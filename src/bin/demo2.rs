extern crate rand;
extern crate rustyballs;
use rand::Rng; // why did i need this for rng.gen?
use rustyballs::render_loop;
use rustyballs::vec3::Vec3;
use rustyballs::color3f::Color3f;
use rustyballs::quaternion::Quaternion;
use rustyballs::raytracer::{random_vector_in_hemisphere,random_normal,VacuumAction,IsectFrom,Ray,RayIsect,RenderConfig,SceneObj,Primitive,
Scene,Material,EPSILON};

const planet_pos: Vec3 = Vec3{x:0., y: 0., z: -4.};

fn atmosphere_scatter_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(Ray{
        origin: isect.hit_pos(),
        dir: random_normal(rng) //(isect.ray.dir.smul(10. * rng.gen::<f32>()) + random_vector_in_hemisphere(&isect.ray.dir, rng)).normal()
    })
}
fn atmosphere_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:0.5, g:0.5, b:1.}, Color3f::black()) }
static scatterDummyObj: SceneObj = SceneObj {
    prim: Primitive::ScatterEvent,
    mat: Material { color_program: atmosphere_cp, path_program: atmosphere_scatter_pp }
};
fn vacuum_program<'a>(isect: &RayIsect, rng: &mut rand::ThreadRng) -> VacuumAction<'a> {
    const SEGMENT_LEN: f32 = 0.1;
    let mut p: f32 = 0.;

    loop {
        let sample_dist = p + SEGMENT_LEN*rng.gen::<f32>();
        if sample_dist >= isect.dist-EPSILON {
            break;
        }
        let point: Vec3 = planet_pos - isect.ray.origin - isect.ray.dir.smul(sample_dist);
        let distance_from_planet = point.length() - 1.5 /*planet radius*/;

        if rng.gen::<f32>() < (-1.-8.*distance_from_planet).exp() {
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

fn black_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f::black(), Color3f::black()) }
fn black_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> { None }
fn star_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:1., b:0.8}, Color3f{r:1., g:1., b:0.8}) }
fn star_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(isect.new_random_ray(rng))
}
fn planet_cp(_: &RayIsect) -> (Color3f, Color3f) { (Color3f{r:1., g:1., b:1.}, Color3f::default()) }
fn planet_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(isect.new_random_ray(rng))
}
fn main() {
    let mut scene: Scene = Scene{
        camera_position: Vec3{x:0., y:0., z:0.},
        camera_orientation: Quaternion::default(),
        objs: Vec::new(),
        vacuum_program: Some(vacuum_program)
    };

    scene.objs = vec![
        SceneObj {
            prim: Primitive::Sphere(planet_pos, 1.5),
            mat: Material { color_program: planet_cp, path_program: planet_pp }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:8.2, y: 0., z: -9.}, 3.),
            mat: Material { color_program: star_cp, path_program: star_pp }
        },
        // black background object so isects do occur (otherwise limb of atmosphere won't render)
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:0., y:0., z:-20.}, 15.),
            mat: Material {color_program: black_cp, path_program: black_pp }
        },
    ];

    render_loop(4, &RenderConfig { threads:8, samples_per_first_isect: 1000, image_size: (512, 512) }, &scene);
}
