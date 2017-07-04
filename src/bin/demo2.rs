extern crate rand;
extern crate rustyballs;
use rand::Rng; // why did i need this for rng.gen?
use rustyballs::render_scene;
use rustyballs::vec3::Vec3;
use rustyballs::color3f::Color3f;
use rustyballs::quaternion::Quaternion;
use rustyballs::shaders::{random_vector_in_hemisphere,random_normal,diffuse_pp};
use rustyballs::raytracer::{ColorProgramResult,Camera,VacuumAction,IsectFrom,Ray,RayIsect,RenderConfig,SceneObj,Primitive,Scene,Material,EPSILON};

const planet_pos: Vec3 = Vec3{x:0., y: 0., z: -4.};

fn atmosphere_scatter_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(Ray{
        origin: isect.hit_pos(),
        dir: random_normal(rng) //(isect.ray.dir.smul(10. * rng.gen::<f32>()) + random_vector_in_hemisphere(&isect.ray.dir, rng)).normal()
    })
}
fn atmosphere_cp(_: &RayIsect) -> ColorProgramResult {
    ColorProgramResult {
        transmissive: Color3f{r:0.5, g:0.5, b:1.}, emissive: Color3f::black()
    }
}
static scatterDummyObj: SceneObj = SceneObj {
    prim: Primitive::ScatterEvent,
    mat: Material { color_program: atmosphere_cp, path_program: atmosphere_scatter_pp, vacuum_program: None }
};
fn atmosphere_scatter_vp<'a>(isect: &RayIsect, rng: &mut rand::ThreadRng) -> VacuumAction<'a> {
    const SEGMENT_LEN: f32 = 0.2;
    let mut p: f32 = 0.;

    match isect.from {
        IsectFrom::Outside => VacuumAction::Continue,
        IsectFrom::Inside => {
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
    }
}
fn transparent_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(Ray{origin: isect.hit_pos() + isect.ray.dir.smul(EPSILON), dir: isect.ray.dir})
}
fn transparent_cp(_: &RayIsect) -> ColorProgramResult {
    ColorProgramResult {
        transmissive: Color3f{r:1., g:1., b:1.},
        emissive: Color3f::default()
    }
}

fn black_cp(_: &RayIsect) -> ColorProgramResult {
    ColorProgramResult {
        transmissive: Color3f::black(),
        emissive: Color3f::black()
    }
}
fn black_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> { None }
fn star_cp(_: &RayIsect) -> ColorProgramResult {
    ColorProgramResult {
        transmissive: Color3f{r:1., g:1., b:0.8},
        emissive: Color3f{r:1., g:1., b:0.8}
    }
}
fn star_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    Some(isect.new_random_ray(rng))
}
fn planet_cp(_: &RayIsect) -> ColorProgramResult {
    ColorProgramResult {
        transmissive: Color3f{r:1., g:1., b:1.},
        emissive: Color3f::default()
    }
}
fn planet_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    diffuse_pp(isect, rng)
}

fn main() {
    let mut scene: Scene = Scene{
        objs: Vec::new(),
    };

    scene.objs = vec![
        // planet
        SceneObj {
            prim: Primitive::Sphere(planet_pos, 1.5),
            mat: Material {
                color_program: planet_cp,
                path_program: planet_pp,
                vacuum_program: Some(atmosphere_scatter_vp)
            }
        },
        // planet atmosphere outer bounds (for vacuum program)
        SceneObj {
            prim: Primitive::Sphere(planet_pos, 3.0),
            mat: Material {
                color_program: transparent_cp,
                path_program: transparent_pp,
                vacuum_program: Some(atmosphere_scatter_vp)
            }
        },
        // star
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:13.3, y: 0., z: -9.}, 3.),
            mat: Material {
                color_program: star_cp,
                path_program: star_pp,
                vacuum_program: None
            }
        },
    ];

    render_scene(
        1000000,
        &RenderConfig { threads:8, samples_per_first_isect: 100, preview_hdr_gamma: 1000.0, image_size: (512, 512) },
        &Camera { position: Vec3{x:0., y:0., z:0.}, orientation: Quaternion::default() },
        &scene
    );
}
