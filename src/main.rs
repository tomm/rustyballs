extern crate sdl2;
extern crate rand;
extern crate time;

use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

mod vec3;
mod color3f;
mod raytracer;
use vec3::Vec3;
use color3f::Color3f;
use raytracer::{SceneObj,Primitive,Material,Ray,RayIsect,Path,MAX_BOUNCES};

const REUSE_EYE_RAY_TIMES: i32 = 20;

fn ray_primitive_intersects(ray: &Ray, scene_obj: &SceneObj) -> Option<RayIsect> {
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
                        Some(RayIsect{dist:i2, scene_obj:scene_obj.clone(), ray:ray.clone()})
                    } else {
                        Some(RayIsect{dist:i1, scene_obj:scene_obj.clone(), ray:ray.clone()})
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        // not implemented yet!
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
                    Some(RayIsect{dist:dist, scene_obj: scene_obj.clone(), ray: ray.clone()})
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

fn find_first_intersection(ray: &Ray, scene: &Vec<SceneObj>) -> Option<RayIsect> {
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

fn isect_pos(isect: &RayIsect) -> Vec3 {
    isect.ray.origin + isect.ray.dir.smul(isect.dist)
}

fn isect_normal(isect: &RayIsect) -> Vec3 {
    match isect.scene_obj.prim {
        Primitive::Sphere(origin, _) => (isect_pos(isect) - origin).normal(),
        Primitive::Triangle(v1, v2, v3) => (v2-v1).cross(&(v2-v3)).normal()
    }
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

const EPSILON: f32 = 0.0001;

fn new_random_ray_from_isect(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Ray {
    let last_isect_norm = isect_normal(isect);
    let ray_start_pos = isect_pos(isect) + last_isect_norm.smul(EPSILON);
    let rand_dir = random_vector_in_hemisphere(&last_isect_norm, rng);
    Ray {origin: ray_start_pos, dir: rand_dir}
}

fn make_ray_scatter_path(ray: &Ray, scene: &Vec<SceneObj>, rng: &mut rand::ThreadRng, path: &mut Path) {
    match find_first_intersection(ray, scene) {
        Some(isect) => {
            path.isects[path.num_bounces as usize] = isect.clone();
            path.num_bounces += 1;
            if path.num_bounces < MAX_BOUNCES as i32 {
                let next_ray = new_random_ray_from_isect(&isect, rng);
                make_ray_scatter_path(&next_ray, scene, rng, path);
            }
        },
        None => ()
    }
}

fn collect_light_from_path(path: &Path) -> Color3f {
    let mut color = Color3f::default();

    for i in (0..path.num_bounces as usize).rev() {
        let surface_normal = isect_normal(&path.isects[i]);
        let cos_theta = (-path.isects[i].ray.dir.normal()).dot(&surface_normal);
        let reflected = (color * path.isects[i].scene_obj.mat.diffuse).smul(cos_theta);

        color = path.isects[i].scene_obj.mat.emissive + reflected;
    }

    color
}

fn path_trace_rays(rays: &Vec<Ray>, scene: &Vec<SceneObj>, rng: &mut rand::ThreadRng, photon_buffer: &mut [Color3f]) {
    for i in 0..rays.len() {
        for _ in 0..REUSE_EYE_RAY_TIMES {
            let mut path = Path::default();
            make_ray_scatter_path(&rays[i], scene, rng, &mut path);
            photon_buffer[i] += collect_light_from_path(&path);
        }
    }
}

fn make_eye_rays(width: i32, height: i32, y_bounds: (i32, i32), sub_pix: (f32, f32)) -> Vec<Ray> {
    let fw = width as f32;
    let fh = height as f32;
    let aspect = fw / fh;
    let top_left_2 = Vec3 {x:-aspect, y:1., z:-1.};
    let right_step = Vec3 {x:2.*aspect, y:0., z:0.}.smul(1. / (fw-1.));
    let down_step = Vec3 {x:0., y:-2., z:0.}.smul(1. / (fh-1.));
    let top_left = top_left_2 + right_step.smul(sub_pix.0) + down_step.smul(sub_pix.1);

    let mut rays = Vec::new();

    for y in y_bounds.0..y_bounds.1 {
        for x in 0..width {
            rays.push(Ray{
                origin: Vec3::default(),
                dir: (top_left + right_step.smul(x as f32) + down_step.smul(y as f32)).normal()
            });
        }
    }
    rays
}

fn path_trace_scene(scene: &Vec<SceneObj>, width: i32, height: i32,
                  y_bounds: (i32, i32), photon_buffer: &mut[Color3f],
                  rng: &mut rand::ThreadRng) {
    let subpix = (rng.gen::<f32>(), rng.gen::<f32>());
    let eye_rays = make_eye_rays(width, height, y_bounds, subpix);

    assert!(eye_rays.len() == photon_buffer.len());

    path_trace_rays(
        &eye_rays,
        scene, rng, photon_buffer
    );
}

fn render_pixels<F>(renderer: &mut sdl2::render::Renderer, photon_buffer: &[Color3f],
                   mut color_transform_fn: F)
    where F: FnMut(&Color3f) -> Color3f {
    
    let output_size = renderer.output_size().unwrap();

    for y in 0..output_size.0 {
        for x in 0..output_size.1 {
            let col = color_transform_fn(&photon_buffer[(x + output_size.0*y) as usize]);
            renderer.set_draw_color(Color::RGB(
                (255.*col.r) as u8,
                (255.*col.g) as u8,
                (255.*col.b) as u8
            ));
            renderer.fill_rect(Rect::new(x as i32, y as i32, 1, 1)).unwrap();
        }
    }
}

fn max3<T: std::cmp::PartialOrd>(a: T, b: T, c: T) -> T {
    if a>b { if a>c {a} else {c} } else { if b>c {b} else {c} }
}

fn hdr_postprocess_blit(renderer: &mut sdl2::render::Renderer, photon_buffer: &[Color3f]) {
    let mut max_color = Color3f::default();

    for col in photon_buffer.iter() {
        if col.r > max_color.r { max_color.r = col.r; }
        if col.g > max_color.g { max_color.g = col.g; }
        if col.b > max_color.b { max_color.b = col.b; }
    }

    let brightness = 1. / max3(max_color.r, max_color.g, max_color.b).sqrt();

    render_pixels(renderer, photon_buffer, |c: &Color3f| {
        Color3f {r: c.r.sqrt(), g: c.g.sqrt(), b: c.b.sqrt()}.smul(brightness)
    });
}

fn main_loop(sdl_context: &sdl2::Sdl, renderer: &mut sdl2::render::Renderer) {
    let scene: Vec<SceneObj> = vec![
        // balls in scene
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:0., y: -1.5, z: -4.}, 1.),
            mat: Material {
                emissive: Color3f {r:0., g:0., b:0.},
                diffuse: Color3f {r:1., g:1., b:1.}
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 2., y:-1., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:0.,g:1.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: -2., y:-1., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:1.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:0., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:1.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        // floor
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: -100., y:-2., z: 0.},
                                      Vec3 {x: -100., y:-2., z: -100.},
                                      Vec3 {x: 100., y:-2., z: 0.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: 100., y:-2., z: 0.},
                                      Vec3 {x: -100., y:-2., z: -100.},
                                      Vec3 {x: 100., y:-2., z: -100.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        // back wall
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: -100., y:-2., z: -10.},
                                      Vec3 {x: 100., y:100., z: -10.},
                                      Vec3 {x: 100., y:-2., z: -10.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: 100., y:100., z: -20.},
                                      Vec3 {x: -100., y:100., z: -10.},
                                      Vec3 {x: -100., y:-2., z: -10.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        },
        /*
        SceneObj {
            prim: Primitive::Triangle(
    {
        Triangle(Vec3{-100,-2,0}, Vec3{-100,-2,-100}, Vec3{100,-2,0}),
        Material {Color{0,0,0}, Color{1,1,1}}
    }, {
        Triangle(Vec3{100,-2,0}, Vec3{-100,-2,-100}, Vec3{100,-2,-100}),
        Material {Color{0,0,0}, Color{1,1,1}}
    },
    // back wall
    {
        Triangle(Vec3{-100,-2,-10}, Vec3{100,100,-10}, Vec3{100,-2,-10}),
        Material{Color{}, Color{1,1,1}}
    }, {
        Triangle(Vec3{100,100,-20},Vec3{-100,100,-10},Vec3{-100,-2,-10}),
        Material{Color{}, Color{1,1,1}}
    },
    */
        // light
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:8., z: -4.}, 3.),
            mat: Material {
                emissive: Color3f {r:1.,g:1.,b:0.8},
                diffuse: Color3f{r:1.,g:1.,b:1.}
            }
        }
        /*
        }, {
            Sphere(Vec3 {-2,-1,-4}, 0.5),
            Material { Color{1,0,0}, Color{1,1,1} }
        }, {
            Sphere(Vec3 {0,0,-4}, 0.5),
            Material { Color{0,0,1}, Color{1,1,1} }
        },
        */
    ];

    /*
    renderer.set_draw_color(Color::RGB(255, 0, 0));
    renderer.clear();
    renderer.set_draw_color(Color::RGB(0, 255, 0));
    match renderer.fill_rect(Rect::new(200, 200, 10, 20)) {
        Ok(_) => {},
        Err(e) => panic!("fill_rect failed: {}", e)
    }
    */

    let output_size = renderer.output_size().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut rng = rand::thread_rng();
    let mut photon_buffer = vec![Color3f::default(); (output_size.0 * output_size.1) as usize];

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        let t = time::precise_time_ns();

        path_trace_scene(&scene, output_size.0 as i32, output_size.1 as i32,
                       (0, output_size.1 as i32),
                       &mut photon_buffer,
                       &mut rng);

        hdr_postprocess_blit(renderer, &photon_buffer);

        let t_ = time::precise_time_ns();
        println!("{} ms per frame, {} paths per second.",
                 (t_ - t)/1000000,
                 ((1000000000u64 * (output_size.0 * output_size.1 * REUSE_EYE_RAY_TIMES as u32) as u64) / (t_ - t))
        );
        renderer.present();
    }
}

use rand::Rng; // why did i need this for rng.gen?

fn main() {
    let mut rng = rand::thread_rng();
    println!("{}\n", rng.gen::<f32>());
    println!("{} {} {}\n",
             rand::random::<f32>(),
             rand::random::<f32>(),
             rand::random::<f32>());
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("RUSTY BALLS!!", 512, 512)
        .position_centered()
        //.opengl()
        .build()
        .unwrap();
    let mut renderer = window.renderer().build().unwrap();

    main_loop(&sdl_context, &mut renderer);
}

