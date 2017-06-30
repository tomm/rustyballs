extern crate sdl2;
extern crate rand;
extern crate time;
extern crate crossbeam;

use std::mem;
use std::slice;
use std::fs::File;
use std::io::prelude::*;
use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use rand::Rng; // why did i need this for rng.gen?

pub mod quaternion;
pub mod vec3;
pub mod color3f;
pub mod raytracer;
use quaternion::Quaternion;
use vec3::Vec3;
use color3f::Color3f;
use raytracer::{VacuumAction,EPSILON,RenderConfig,SceneObj,Primitive,Material,Ray,RayIsect,Scene,
Path,IsectFrom,MAX_BOUNCES};

fn ray_primitive_intersects<'a>(ray: &Ray, scene_obj: &'a SceneObj) -> Option<RayIsect<'a>> {
    match scene_obj.prim {
        Primitive::Sphere(origin, radius) => {
            let v = ray.origin - origin;
            let b = -(v.dot(&ray.dir));
            let sq_det = (b*b) - v.dot(&v) + radius*radius;
            if sq_det > 0. {
                let det = sq_det.sqrt();
                let i1 = b - det;
                let i2 = b + det;
                if i2 > 0. {
                    if i1 < 0. {
                        // inside! XXX don't need until we have refraction
                        Some(RayIsect{from: IsectFrom::Inside, dist:i2, scene_obj:&scene_obj, ray:ray.clone()})
                    } else {
                        // outside
                        Some(RayIsect{from: IsectFrom::Outside, dist:i1, scene_obj:&scene_obj, ray:ray.clone()})
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        Primitive::Triangle(a, b, c) => {
            let n = (c-a).cross(&(b-a));
            let v0_cross = (b-ray.origin).cross(&(a-ray.origin));
            let v1_cross = (a-ray.origin).cross(&(c-ray.origin));
            let v2_cross = (c-ray.origin).cross(&(b-ray.origin));
            let nominator = n.dot(&(a-ray.origin));
            let v0d = v0_cross.dot(&ray.dir);
            let v1d = v1_cross.dot(&ray.dir);
            let v2d = v2_cross.dot(&ray.dir);
            if (v0d > 0. && v1d > 0. && v2d > 0.) ||
               (v0d < 0. && v1d < 0. && v2d < 0.) {
                let dist = nominator / ray.dir.dot(&n);
                if dist > EPSILON {
                    Some(RayIsect{from: IsectFrom::Outside, dist:dist, scene_obj: &scene_obj, ray: ray.clone()})
                } else {
                    None
                }
            } else {
                None
            }
        }
        Primitive::Plane(center, normal) => {
            let denom = normal.dot(&ray.dir);
            if denom.abs() > EPSILON {
                let t: f32 = (center - ray.origin).dot(&normal) / denom;
                if t >= 0. {
                    Some(RayIsect{from: IsectFrom::Outside, dist:t, scene_obj: &scene_obj, ray: ray.clone()})
                } else {
                    None
                }
            } else {
                None
            }
        }
        Primitive::ScatterEvent => None
    }
}

fn find_first_intersection<'a>(ray: &Ray, scene: &'a Vec<SceneObj>) -> Option<RayIsect<'a>> {
    let mut nearest: Option<RayIsect> = None;

    for obj in scene {
        match ray_primitive_intersects(&ray, &obj) {
            Some(isect) => {
                if nearest.is_some() {
                    let nearest_isect = nearest.unwrap();
                    if isect.dist < nearest_isect.dist {
                        nearest = Some(isect);
                    } else {
                        nearest = Some(nearest_isect);
                    }
                } else {
                    nearest = Some(isect);
                }
            },
            None => {}
        }
    }
    nearest
}

fn flip_vector_to_hemisphere(flipee: &Vec3, norm: &Vec3) -> Vec3 {
    if flipee.dot(norm) > 0. {
        *flipee
    } else {
        -*flipee
    }
}

fn random_vector_in_hemisphere(norm: &Vec3, rng: &mut rand::ThreadRng) -> Vec3 {
    flip_vector_to_hemisphere(
        &Vec3 {x: 0.5-rng.gen::<f32>(), y: 0.5-rng.gen::<f32>(), z: 0.5-rng.gen::<f32>()},
        norm
    ).normal()
}

fn new_random_ray_from_isect(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Ray {
    let last_isect_norm = isect.normal();
    let ray_start_pos = isect.hit_pos() + last_isect_norm.smul(EPSILON);
    let rand_dir = random_vector_in_hemisphere(&last_isect_norm, rng);
    Ray {origin: ray_start_pos, dir: rand_dir}
}

fn make_ray_scatter_path<'a>(ray: &Ray, scene: &'a Scene, rng: &mut rand::ThreadRng, path: &mut Path<'a>) {
    match find_first_intersection(ray, &scene.objs) {
        Some(isect) => {
            path.isects[path.num_bounces as usize] = isect.clone();
            match scene.vacuum_program {
                Some(prog) => {
                    match (prog)(&isect, rng) {
                        VacuumAction::Continue => {},
                        VacuumAction::Scatter(new_isect) => {
                            // replace intersection with a scattering
                            path.isects[path.num_bounces as usize] = new_isect.clone();
                        }
                    }
                },
                None => {}
            };
            path.num_bounces += 1;
            if path.num_bounces < MAX_BOUNCES as i32 {
                // call material's path_program to see what our next ray will be
                match (isect.scene_obj.mat.path_program)(&isect, rng) {
                    Some(next_ray) => {
                        make_ray_scatter_path(&next_ray, scene, rng, path);
                    },
                    None => {}
                }
            }
        },
        None => ()
    }
}

fn collect_light_from_path(path: &Path) -> Color3f {
    let mut color = Color3f::default();

    for i in (0..path.num_bounces as usize).rev() {
        let surface_normal = &path.isects[i].normal();
        let (transmissive, emissive) = (path.isects[i].scene_obj.mat.color_program)(&path.isects[i]);
        let mut cos_theta;
        
        if i < (path.num_bounces-1) as usize {
            cos_theta = match path.isects[i+1].from {
                IsectFrom::Inside => (-path.isects[i+1].ray.dir).dot(&surface_normal),
                IsectFrom::Outside => path.isects[i+1].ray.dir.dot(&surface_normal)
            };
        } else {
            cos_theta = 1.;
        }
        if cos_theta < 0. { cos_theta = 0.; }

        color = emissive + (color * transmissive).smul(cos_theta);
    }

    color
}

fn path_trace_rays(config: &RenderConfig, rays: &Vec<Ray>, scene: &Scene,
                   rng: &mut rand::ThreadRng, photon_buffer: &mut [Color3f]) {

    let mut path = Path {
        num_bounces: 0,
        isects: [RayIsect{from: IsectFrom::Outside, ray:Ray::default(), dist: 0., scene_obj: &scene.objs[0]}; MAX_BOUNCES]
    };
    // could have initted unsafely (and maybe unwisely) like this also:
    // unsafe { path = std::mem::uninitialized(); }

    for i in 0..rays.len() {
        // trace first path and collect its light contribution
        path.num_bounces = 0;
        make_ray_scatter_path(&rays[i], scene, rng, &mut path);
        photon_buffer[i] += collect_light_from_path(&path);
        // now reuse the first isect for a few more paths! (great optimisation)
        if path.num_bounces > 0 {
            let first_isect = path.isects[0].clone();
            for _ in 0..config.samples_per_first_isect {
                path.num_bounces = 1;
                match (first_isect.scene_obj.mat.path_program)(&first_isect, rng) {
                    Some(next_ray) => {
                        make_ray_scatter_path(&next_ray, scene, rng, &mut path);
                        photon_buffer[i] += collect_light_from_path(&path);
                    },
                    None => {}
                }
            }
        }
    }
}

fn make_eye_rays(camera_position: &Vec3, camera_orientation: &Quaternion,
                 width: i32, height: i32, y_bounds: (i32, i32), sub_pix: (f32, f32)) -> Vec<Ray> {
    let fw = width as f32;
    let fh = height as f32;
    let aspect = fw / fh;
    let fov = 3.14 / 2.0;
    let top_left_2 = Vec3 {x:-aspect, y:1., z:-1.};
    let right_step = Vec3 {x:2.*aspect, y:0., z:0.}.smul(1. / (fw-1.));
    let down_step = Vec3 {x:0., y:-2., z:0.}.smul(1. / (fh-1.));
    let top_left = top_left_2 + right_step.smul(sub_pix.0) + down_step.smul(sub_pix.1);

    let mut rays = Vec::new();

    for y in y_bounds.0..y_bounds.1 {
        for x in 0..width {
            rays.push(Ray{
                origin: *camera_position,
                dir: camera_orientation.vmul(
                    &(top_left + right_step.smul(x as f32) + down_step.smul(y as f32)).normal()
                )
            });
        }
    }
    rays
}

fn path_trace_scene(config: &RenderConfig, scene: &Scene, width: i32, height: i32,
                  y_bounds: (i32, i32), photon_buffer: &mut[Color3f],
                  rng: &mut rand::ThreadRng) {
    let subpix = (rng.gen::<f32>(), rng.gen::<f32>());
    let eye_rays = make_eye_rays(&scene.camera_position, &scene.camera_orientation,
                                 width, height, y_bounds, subpix);

    assert!(eye_rays.len() == photon_buffer.len());

    path_trace_rays(
        config,
        &eye_rays,
        scene, rng, photon_buffer
    );
}

fn render_pixels<F>(renderer: &mut sdl2::render::Renderer, photon_buffer: &[Color3f],
                    color_transform_fn: F)
    where F: Fn(&Color3f) -> Color3f {
    
    let output_size = renderer.output_size().unwrap();

    for y in 0..output_size.0 {
        for x in 0..output_size.1 {
            let col = color_transform_fn(&photon_buffer[(x + output_size.0*y) as usize]);
            renderer.set_draw_color(Color::RGB(
                col.r as u8,
                col.g as u8,
                col.b as u8
            ));
            renderer.fill_rect(Rect::new(x as i32, y as i32, 1, 1)).unwrap();
        }
    }
}

fn hdr_postprocess_blit(renderer: &mut sdl2::render::Renderer, photon_buffer: &[Color3f]) {
    let mut max_color = 0.;

    for col in photon_buffer.iter() {
        if col.r > max_color { max_color = col.r; }
        if col.g > max_color { max_color = col.g; }
        if col.b > max_color { max_color = col.b; }
    }

    let brightness = 255. / (1.+max_color).ln();

    render_pixels(renderer, photon_buffer, |c: &Color3f| {
        Color3f {r: (c.r+1.).ln(), g: (c.g+1.).ln(), b: (c.b+1.).ln()}.smul(brightness)
    });
}

fn parallel_path_trace_scene(config: &RenderConfig, scene: &Scene, width: u32, height: u32,
                             photon_buffer: &mut[Color3f]) {
    let chunks: Vec<_> = photon_buffer.chunks_mut((width * height) as usize / config.threads).collect();

    crossbeam::scope(|scope| {
        for (i, chunk) in chunks.into_iter().enumerate() {
            scope.spawn(move || {
                let mut rng = rand::thread_rng();
                path_trace_scene(config,
                                 scene,
                                 width as i32,
                                 height as i32,
                                 ((i*height as usize / config.threads) as i32,
                                  ((i+1)*height as usize / config.threads) as i32),
                                 chunk,
                                 &mut rng);
            });
        }
    });
}

fn normalize_photon_buffer(photon_buffer: &Vec<Color3f>) -> (Vec<Color3f>, f32) {
    let mut buf = photon_buffer.clone();

    // normalize colour values to [0..1]
    let mut max_value: f32 = 0.;
    for c in buf.iter() {
        if c.max_channel() > max_value {
            max_value = c.max_channel()
        }
    }
    for c in buf.iter_mut() {
        *c = Color3f {r: c.r / max_value, g: c.g / max_value, b: c.b / max_value};
    }
    (buf, max_value)
}

fn save_photon_buffer(stat_samples: u32, img_size: (u32, u32), photon_buffer: &Vec<Color3f>) {
    let (buf, max_value) = normalize_photon_buffer(photon_buffer);

    let t = time::precise_time_ns();

    {
        // write raw float data
        let filename = format!("img_{}_{}_samples.raw", t, stat_samples);
        println!("Writing raw float32 image data to {}", filename);
        let mut f = match File::create(&filename) {
            Ok(file) => file,
            Err(err) => return
        };


        // need to dirty-cast our Vec<Color3f> to [u8]
        let _c: *const Color3f = &buf[0];
        let _u: *const u8 = _c as *const _;
        let us: &[u8] = unsafe {
            slice::from_raw_parts(_u, buf.len() * mem::size_of::<Color3f>())
        };
        f.write_all(us);
        f.sync_data();
    }

    {
        let filename = format!("img_{}_{}_samples.ppm", t, stat_samples);
        println!("Writing RGB tone-mapped image to {}", filename);
        let mut f = match File::create(&filename) {
            Ok(file) => file,
            Err(err) => return
        };

        // XXX get proper dimensions!
        f.write_all(format!("P6 {} {} 255\n", img_size.0, img_size.1).as_bytes());

        // write log mapped u8 data (as displayed)
        let brightness = 255. / (1.+max_value).ln();
        let mut rgb: Vec<u8> = Vec::new();
        for c in photon_buffer.iter() {
            rgb.push(((c.r+1.).ln() * brightness) as u8);
            rgb.push(((c.g+1.).ln() * brightness) as u8);
            rgb.push(((c.b+1.).ln() * brightness) as u8);
        }
        f.write_all(&rgb);
        f.sync_data();
    }
}

pub fn render_loop<F>(config: &RenderConfig, mut scene: &mut Scene, mut pre_draw_callback: F)
    where F: FnMut(&mut Scene, &mut Vec<Color3f>) {

    println!("Keys: <esc> to quit, <s> to dump raw image");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("RUSTY BALLS!!", 512, 512)
        .position_centered()
        //.opengl()
        .build()
        .unwrap();
    let mut renderer = window.renderer().build().unwrap();

    let output_size = renderer.output_size().unwrap();
    let mut stats_samples_per_pixel: u32 = 0;
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut photon_buffer = vec![Color3f::default(); (output_size.0 * output_size.1) as usize];

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                    save_photon_buffer(stats_samples_per_pixel, output_size, &photon_buffer);
                }
                _ => {}
            }
        }

        pre_draw_callback(&mut scene, &mut photon_buffer);

        let t = time::precise_time_ns();

        parallel_path_trace_scene(config, &scene, output_size.0, output_size.1, &mut photon_buffer);
        hdr_postprocess_blit(&mut renderer, &photon_buffer);

        let t_ = time::precise_time_ns();
        stats_samples_per_pixel += 1 + config.samples_per_first_isect;
        println!("{} accumulated samples per pixel. {} ms per frame, {} paths per second.",
                 stats_samples_per_pixel,
                 (t_ - t)/1000000,
                 ((1000000000u64 * (output_size.0 * output_size.1 * (1u32+config.samples_per_first_isect)) as u64) / (t_ - t))
        );
        renderer.present();
    }
}
