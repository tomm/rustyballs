extern crate rand;
extern crate rustyballs;
use rand::Rng; // why did i need this for rng.gen?
use rustyballs::render_loop;
use rustyballs::vec3::Vec3;
use rustyballs::color3f::Color3f;
use rustyballs::raytracer::{IsectFrom,Ray,RayIsect,RenderConfig,SceneObj,Primitive,Material,EPSILON};

fn shiny_prog(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let die = rng.gen::<f32>();
    if die < 0.2 {
        let isect_normal = isect.normal();
        let isect_pos = isect.hit_pos();
            let reflect = isect.ray.dir - (isect_normal.smul(isect.ray.dir.dot(&isect_normal))).smul(2.);
            Some(Ray{origin: isect_pos + isect_normal.smul(EPSILON),
                     dir: reflect.normal()})
    } else {
        // diffuse
        Some(isect.new_random_ray(rng))
    }
}

fn glass_prog(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let isect_normal = isect.normal();
    let isect_pos = isect.hit_pos();

    // refraction
    let norm: Vec3;
    // refractive index of original medium
    let n1: f32;
    let n2: f32;
    
    match isect.from {
        IsectFrom::Outside => {
            n1 = 1.; n2 = 1.4;
            norm = isect_normal;
        },
        IsectFrom::Inside => {
            n1 = 1.4; n2 = 1.;
            norm = -isect_normal;
        }
    };
    // incoming angle too tight: reflect instead
    if 1.+norm.dot(&isect.ray.dir) > rng.gen::<f32>() {
        let reflect = isect.ray.dir - (isect_normal.smul(isect.ray.dir.dot(&isect_normal))).smul(2.);
        return Some(Ray{origin: isect_pos + isect_normal.smul(EPSILON),
                 dir: reflect.normal()});
    }
    let n: f32 = n1 / n2;
    let c1 = -norm.dot(&isect.ray.dir);
    let c2 = (1. - n*n * (1. - c1*c1)).sqrt();
    let reflect_dir = (isect.ray.dir.smul(n) + norm.smul(n * c1 - c2)).normal();
    // XXX sometimes this fails. could be on very edge of shape. investigate
    //assert!(isect.ray.dir.dot(&reflect_dir) >= 0.);

    Some(Ray{origin: isect_pos - norm.smul(EPSILON),
             dir: reflect_dir})
}

fn main() {
    let mut scene: Vec<SceneObj> = vec![
        // balls in scene
        SceneObj {
            prim: Primitive::Sphere(Vec3{x:0., y: -0.6, z: -4.}, 1.),
            mat: Material {
                emissive: Color3f {r:0., g:0., b:0.},
                diffuse: Color3f {r:1., g:1., b:1.},
                isect_prog: glass_prog
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 2., y:-1., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:0.,g:1.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: glass_prog
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: -2., y:-1., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:1.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: glass_prog
            }
        },
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:0., z: -4.}, 0.5),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:1.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: glass_prog
            }
        },
        // floor
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: -100., y:-2., z: 0.},
                                      Vec3 {x: -100., y:-2., z: -100.},
                                      Vec3 {x: 100., y:-2., z: 0.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: shiny_prog
            }
        },
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: 100., y:-2., z: 0.},
                                      Vec3 {x: -100., y:-2., z: -100.},
                                      Vec3 {x: 100., y:-2., z: -100.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: shiny_prog
            }
        },
        // back wall
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: -100., y:-2., z: -10.},
                                      Vec3 {x: 100., y:100., z: -10.},
                                      Vec3 {x: 100., y:-2., z: -10.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: shiny_prog
            }
        },
        SceneObj {
            prim: Primitive::Triangle(Vec3 {x: 100., y:100., z: -20.},
                                      Vec3 {x: -100., y:100., z: -10.},
                                      Vec3 {x: -100., y:-2., z: -10.}),
            mat: Material {
                emissive: Color3f {r:0.,g:0.,b:0.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: shiny_prog
            }
        },
        // light
        SceneObj {
            prim: Primitive::Sphere(Vec3 {x: 0., y:8., z: -4.}, 1.),
            mat: Material {
                emissive: Color3f {r:1.,g:1.,b:1.},
                diffuse: Color3f{r:1.,g:1.,b:1.},
                isect_prog: shiny_prog
            }
        }
    ];

    let mut time: f32 = 0.;
    render_loop(&RenderConfig{threads:8, samples_per_first_isect: 20},
                scene, |scene, photon_buffer| {
                    /*
        time += 0.1;
        scene[0].prim = Primitive::Sphere(Vec3{x:0., y: -0.6 + time.sin(), z: -4.}, 1.);
        scene[1].prim = Primitive::Sphere(Vec3 {x: 2.*time.sin(), y:-1., z: -4.-2.*time.cos()}, 0.5);
        scene[2].prim = Primitive::Sphere(Vec3 {x: 2.*(time+3.1416).sin(), y:-1., z: -4.-2.*(time+3.1416).cos()}, 0.5);

        // wipe the photon buffer since objects have moved and we don't want a smear of colour
        for c in photon_buffer.iter_mut() { *c = Color3f::default() }
        */
    });
}
