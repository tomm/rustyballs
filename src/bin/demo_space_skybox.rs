extern crate rand;
extern crate noise;
extern crate rustyballs;
#[macro_use]
extern crate lazy_static;

use noise::NoiseModule;
use rand::Rng; // why did i need this for rng.gen?
use rustyballs::render_scene;
use rustyballs::dump_hdr_postprocessed_image;
use rustyballs::max_value_of_photon_buffer;
use rustyballs::vec3::Vec3;
use rustyballs::color3f::Color3f;
use rustyballs::quaternion::Quaternion;
use rustyballs::raytracer::{random_vector_in_hemisphere,random_normal,VacuumAction,IsectFrom,Ray,RayIsect,RenderConfig,SceneObj,Primitive,
Scene,Camera,Material,EPSILON};

static ITERS: i32 = 500;
static RESOLUTION: (u32, u32) = (1024, 1024);

lazy_static! {
    static ref perlin: noise::Perlin = noise::Perlin::new();
    static ref gas_giant_basis: [Vec3; 3] = {
        let pole = Vec3{x:0.5, y:0., z:1.0}.normal();
        let a = Vec3{x:1., y:0., z:0.}.cross(&pole).normal();
        let b = a.cross(&pole).normal();
        [pole, a, b]
    };
}

fn perlin3d(p: &Vec3) -> f32 {
    perlin.get([p.x, p.y, p.z])
}
fn perlin1d(p: f32) -> f32 {
    perlin.get([p,0.0])
}
fn octavenoise(octaves: i32, persistence: f32, lacunarity: f32, p: &Vec3) -> f32
{
    let mut n: f32 = 0.0;
	let mut octaveAmplitude: f32 = 1.0;
	let mut jizm: f32 = 1.0;
    for i in 0..octaves {
		n += octaveAmplitude * perlin3d(&p.smul(jizm));
		octaveAmplitude *= persistence;
		jizm *= lacunarity;
	}
	(0.5 + n*0.5)
}

// _pp = PathProgram
fn semi_mirror_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let die = rng.gen::<f32>();
    if die < 0.5 {
        let isect_normal = isect.normal();
        let isect_pos = isect.hit_pos();
            let reflect = isect.ray.dir - (isect_normal.smul(isect.ray.dir.dot(&isect_normal))).smul(2.);
            Some(Ray{origin: isect_pos + isect_normal.smul(EPSILON),
                     dir: reflect.normal()})
    } else {
        // transmissive
        Some(isect.new_random_ray(rng))
    }
}

fn glass_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let isect_normal = isect.normal();
    let isect_pos = isect.hit_pos();

    let die = rng.gen::<f32>();
    if die < 0.05 {
        // reflection
        let reflect = isect.ray.dir - (isect_normal.smul(isect.ray.dir.dot(&isect_normal))).smul(2.);
        Some(Ray{origin: isect_pos + isect_normal.smul(EPSILON),
                 dir: reflect.normal()})
    } else {
        // refraction
        let norm: Vec3;
        // refractive index of original medium
        let n1: f32;
        let n2: f32;
        
        match isect.from {
            IsectFrom::Outside => {
                n1 = 1.; n2 = 1.5;
                norm = isect_normal;
            },
            IsectFrom::Inside => {
                n1 = 1.5; n2 = 1.;
                norm = -isect_normal;
            }
        };
        let n: f32 = n1 / n2;
        let c1 = -norm.dot(&isect.ray.dir);
        let c2 = (1. - n*n * (1. - c1*c1)).sqrt();
        let refract_dir = (isect.ray.dir.smul(n) + norm.smul(n * c1 - c2)).normal();
        // XXX sometimes this fails. could be on very edge of shape. investigate
        //assert!(isect.ray.dir.dot(&refract_dir) >= 0.);

        Some(Ray{origin: isect_pos - norm.smul(EPSILON),
                 dir: refract_dir})
    }
}
fn null_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    None
}
fn fog_scatter_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(Ray{
        origin: isect.hit_pos(),
        dir: (isect.ray.dir.smul(10. * rng.gen::<f32>()) + random_vector_in_hemisphere(&isect.ray.dir, rng)).normal()
    })
}
fn diffuse_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let norm = isect.normal();
    Some(Ray{
        origin: isect.hit_pos() + norm.smul(EPSILON),
        dir: random_vector_in_hemisphere(&isect.normal(), rng)
    })
}

fn gas_giant_ring_isect_radius(isect: &RayIsect) -> f32 {
    match isect.scene_obj.prim {
        Primitive::Plane(pos, normal) => {
            let isect_hitpos = isect.hit_pos();
            let dir = isect_hitpos - pos;
            normal.cross(&dir).length()
        }
        _ => unreachable!()
    }
}
fn gas_giant_ring_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    match isect.scene_obj.prim {
        Primitive::Plane(pos, normal) => {
            let dist = gas_giant_ring_isect_radius(isect);
            if dist > 0.85 && dist < 1.2 {
                diffuse_pp(isect, rng)
            } else {
                // missed rings. continue on
                Some(Ray{origin: isect.hit_pos() + normal.smul(EPSILON), dir: isect.ray.dir})
            }
        }
        _ => unreachable!()
    }
}
fn gas_giant_ring_cp(isect: &RayIsect) -> (Color3f, Color3f) {
    match isect.scene_obj.prim {
        Primitive::Plane(pos, normal) => {
            let dist = gas_giant_ring_isect_radius(isect);
            if dist > 0.85 && dist < 1.2 {
                let mut brightness =
                    perlin3d(&Vec3 {x: 0., y: 0., z: 30.0*dist}) +
                    perlin3d(&Vec3 {x: 0., y: 0., z: 60.0*dist}) +
                    perlin3d(&Vec3 {x: 0., y: 0., z: 120.0*dist});
                brightness *= brightness;
                (Color3f{r:brightness, g:brightness, b:brightness}, Color3f::default())
            } else {
                (Color3f{r:1.0, g:1.0, b:1.0}, Color3f::default())
            }
        }
        _ => unreachable!()
    }
}
fn gas_giant_cp(isect: &RayIsect) -> (Color3f, Color3f) {
    match isect.scene_obj.prim {
        Primitive::Sphere(pos, radius) => {
            let p = (isect.hit_pos() - pos).normal();
            let q = Vec3{x: p.dot(&gas_giant_basis[0]), y: p.dot(&gas_giant_basis[1]), z: p.dot(&gas_giant_basis[2])};
            let n = octavenoise(12, 0.5, 2.0, &q.smul(perlin3d(&Vec3{x:q.x*10.0, y:q.y*2.0, z:q.z*2.0})));
            (
                Color3f{r:0.50,g:0.22,b:0.18}.smul(1.0-n)+Color3f{r:0.99,g:0.76,b:0.62}.smul(n),
                Color3f::default() 
            )
        }
        _ => unreachable!()
    }
}
fn moon_cp(isect: &RayIsect) -> (Color3f, Color3f) {
    match isect.scene_obj.prim {
        Primitive::Sphere(pos, radius) => {
            let p = (isect.hit_pos() - pos).normal();
            let n = octavenoise(12, 0.5, 2.0, &p.smul(100.0));
            (
                Color3f{r:0.50,g:0.50,b:0.50}.smul(1.0-n)+Color3f{r:1.0,g:1.0,b:1.0}.smul(n),
                Color3f::default() ,
            )
        }
        _ => unreachable!()
    }
}

// _cp = ColorProgram
// returning (transmissive, emissive) colours
fn white_wall_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:1., b:1.}, Color3f::default()) }
fn left_wall_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:0., b:0.}, Color3f::default()) }
fn right_wall_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:0., g:0., b:1.}, Color3f::default()) }
fn red_ball_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:0.5, b:0.5}, Color3f::default()) }
fn green_ball_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:0.5, g:1., b:0.5}, Color3f::default()) }
fn blue_ball_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:0.5, g:0.5, b:1.}, Color3f::default()) }
fn bright_white_light_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:1., b:1.}, Color3f{r:1., g:1., b:1.}) }
fn check_floor_cp(isect: &RayIsect) -> (Color3f, Color3f) {
    let pos = isect.hit_pos();
    if ((pos.x.floor() as i32 + pos.z.floor() as i32) & 1) == 0 {
        (Color3f{r:1., g:1., b:1.}, Color3f::default())
    } else {
        (Color3f{r:0.5, g:0.5, b:0.5}, Color3f::default())
    }
}
fn red_star_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:0.3, b:0.1}, Color3f{r:1., g:0.3, b:0.1}) }

fn fog_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:1., b:1.}, Color3f::black()) }
static scatterDummyObj: SceneObj = SceneObj {
    prim: Primitive::ScatterEvent,
    mat: Material { color_program: fog_cp, path_program: fog_scatter_pp }
};

fn vacuum_program<'a>(isect: &RayIsect, rng: &mut rand::ThreadRng) -> VacuumAction<'a> {
    return VacuumAction::Continue;
    /*
    match isect.from {
        // no fog inside objects!
        IsectFrom::Inside => VacuumAction::Continue,
        IsectFrom::Outside => {
            const SEGMENT_LEN: f32 = 1.0;
            let mut p: f32 = 0.;
            // kind of ray-marching through the fog
            while p+SEGMENT_LEN < isect.dist {
                let sample_dist = p + SEGMENT_LEN*rng.gen::<f32>();
                let point: Vec3 = isect.ray.origin + isect.ray.dir.smul(sample_dist);
                if rng.gen::<f32>() > 0.9 && point.y > 2. {
                    return VacuumAction::Scatter(
                        // return new isect to replace 'isect'
                        RayIsect {
                            ray: isect.ray.clone(),
                            // isect at random location in this segment
                            dist: sample_dist,
                            from: IsectFrom::Outside,
                            scene_obj: &scatterDummyObj
                        },
                    )
                }
                p += SEGMENT_LEN;
            }
            VacuumAction::Continue
        }
    }
    */
}

fn main() {
    let mut scene: Scene = Scene{
        objs: Vec::new(),
        vacuum_program: Some(vacuum_program)
    };
    scene.objs = vec![
        // star to right of camera
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 10., y:-0.8, z: -0.3}, 1.),
            mat: Material { color_program: red_star_cp, path_program: diffuse_pp /* end paths here */ }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 10., y:1.3, z: 1.1}, 1.),
            mat: Material { color_program: red_star_cp, path_program: diffuse_pp /* end paths here */ }
        },
        // moon below camera
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:-0.1, z: 0.}, 0.0995),
            mat: Material { color_program: moon_cp, path_program: diffuse_pp }
        },
        // gas giant in above & front of camera
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:1., z: -1.}, 0.65),
            mat: Material { color_program: gas_giant_cp, path_program: diffuse_pp }
        },
        // gas giant ring
        SceneObj {
            prim: Primitive:: Plane(Vec3 {x: 0., y:1., z: -1.}, Vec3{x:0.5, y:0., z:1.0}.normal()),
            mat: Material { color_program: gas_giant_ring_cp, path_program: gas_giant_ring_pp }
        },
    /*
        // floor
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:0., y:0., z:0.}, Vec3{x:0.,y:1., z:0.}),
            mat: Material { color_program: check_floor_cp, path_program: semi_mirror_pp }
        },
        */
    ];

    let render_config = RenderConfig { threads:8, samples_per_first_isect: 19, image_size: RESOLUTION};

    let camera = Camera {
        position: Vec3{x:0., y:0., z:0.},
        orientation: Quaternion::default()
    };

    render_skybox("vrdemosky", ITERS, &render_config, &camera, &scene);
}

fn render_skybox(file_prefix: &str, iterations: i32, render_config: &RenderConfig, camera: &Camera, scene: &Scene)
{
    let mut face_cam = *camera;

    let img_fr = render_scene(iterations, &render_config, &face_cam, &scene);
    face_cam.orientation = camera.orientation * Quaternion::from_axis_angle(&Vec3{x:1., y:0., z:0.}, 0.5*std::f32::consts::PI);
    let img_up = render_scene(iterations, &render_config, &face_cam, &scene);
    face_cam.orientation = camera.orientation * Quaternion::from_axis_angle(&Vec3{x:1., y:0., z:0.}, -0.5*std::f32::consts::PI);
    let img_dn = render_scene(iterations, &render_config, &face_cam, &scene);
    face_cam.orientation = camera.orientation * Quaternion::from_axis_angle(&Vec3{x:0., y:1., z:0.}, -0.5*std::f32::consts::PI);
    let img_rt = render_scene(iterations, &render_config, &face_cam, &scene);
    face_cam.orientation = camera.orientation * Quaternion::from_axis_angle(&Vec3{x:0., y:1., z:0.}, 0.5*std::f32::consts::PI);
    let img_lf = render_scene(iterations, &render_config, &face_cam, &scene);
    face_cam.orientation = camera.orientation * Quaternion::from_axis_angle(&Vec3{x:0., y:1., z:0.}, std::f32::consts::PI);
    let img_bk = render_scene(iterations, &render_config, &face_cam, &scene);
    
    // to hdr-postprocess all 6 cube faces the same we need to find the max colour value of all of them
    let max_value = max_value_of_photon_buffer(&img_fr).max(
        max_value_of_photon_buffer(&img_up).max(
            max_value_of_photon_buffer(&img_dn).max(
                max_value_of_photon_buffer(&img_rt).max(
                    max_value_of_photon_buffer(&img_lf).max(max_value_of_photon_buffer(&img_bk))
                )
            )
        )
    );

    dump_hdr_postprocessed_image(&format!("{}_fr", file_prefix), render_config.image_size, max_value, &img_fr);
    dump_hdr_postprocessed_image(&format!("{}_bk", file_prefix), render_config.image_size, max_value, &img_bk);
    dump_hdr_postprocessed_image(&format!("{}_up", file_prefix), render_config.image_size, max_value, &img_up);
    dump_hdr_postprocessed_image(&format!("{}_dn", file_prefix), render_config.image_size, max_value, &img_dn);
    dump_hdr_postprocessed_image(&format!("{}_lf", file_prefix), render_config.image_size, max_value, &img_lf);
    dump_hdr_postprocessed_image(&format!("{}_rt", file_prefix), render_config.image_size, max_value, &img_rt);
}
