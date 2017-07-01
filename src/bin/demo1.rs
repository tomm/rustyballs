extern crate rand;
extern crate rustyballs;
use rand::Rng; // why did i need this for rng.gen?
use rustyballs::render_loop;
use rustyballs::vec3::Vec3;
use rustyballs::color3f::Color3f;
use rustyballs::quaternion::Quaternion;
use rustyballs::raytracer::{random_vector_in_hemisphere,random_normal,VacuumAction,IsectFrom,Ray,RayIsect,RenderConfig,SceneObj,Primitive,
Scene,Material,EPSILON};

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
        camera_position: Vec3{x:0.0, y:1.5, z:-1.},
        camera_orientation: 
            Quaternion::from_axis_angle(&Vec3{x:-1., y:0., z:0.}, 0.2),
        objs: Vec::new(),
        vacuum_program: Some(vacuum_program)
    };
    scene.objs = vec![
        // light
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:3., z: -3.}, 0.5),
            mat: Material { color_program: bright_white_light_cp, path_program: diffuse_pp /* end paths here */ }
        },
        // balls in scene
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: -1.2, y:0.7, z: -3.}, 0.5),
            mat: Material { color_program: red_ball_cp, path_program: glass_pp }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:0., y: 0.7, z: -3.}, 0.5),
            mat: Material { color_program: green_ball_cp, path_program: glass_pp }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 1.2, y:0.7, z: -3.}, 0.5),
            mat: Material { color_program: blue_ball_cp, path_program: glass_pp }
        },
        // floor
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:0., y:0., z:0.}, Vec3{x:0.,y:1., z:0.}),
            mat: Material { color_program: check_floor_cp, path_program: semi_mirror_pp }
        },
        // back wall
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:0., y:0., z:-6.}, Vec3{x:0.,y:0., z:1.}),
            mat: Material { color_program: white_wall_cp, path_program: diffuse_pp }
        },
        // left wall
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:-2., y:0., z:0.}, Vec3{x:1.,y:0., z:0.}),
            mat: Material { color_program: left_wall_cp, path_program: diffuse_pp }
        },
        // right wall
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:2., y:0., z:0.}, Vec3{x:-1.,y:0., z:0.}),
            mat: Material { color_program: right_wall_cp, path_program: diffuse_pp }
        },
        // roof
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:0., y:3., z:0.}, Vec3{x:0.,y:-1., z:0.}),
            mat: Material { color_program: white_wall_cp, path_program: diffuse_pp }
        },
        // wall behind camera
        SceneObj {
            prim: Primitive::Plane(Vec3 {x:0., y:0., z:0.}, Vec3{x:0.,y:0., z:-1.}),
            mat: Material { color_program: white_wall_cp, path_program: diffuse_pp }
        },
    ];

    render_loop(1000000, &RenderConfig { threads:8, samples_per_first_isect: 19, image_size: (512, 512) }, &scene);
}
