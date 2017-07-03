extern crate rand;
use rand::Rng; // why did i need this for rng.gen?
use raytracer::{EPSILON,Ray,RayIsect};
use vec3::Vec3;

pub fn end_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> { None }

pub fn mirror_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let isect_normal = isect.normal();
    let isect_pos = isect.hit_pos();
    let reflect = isect.ray.dir - (isect_normal.smul(isect.ray.dir.dot(&isect_normal))).smul(2.);
    Some(Ray{origin: isect_pos + isect_normal.smul(EPSILON), dir: reflect.normal()})
}

pub fn diffuse_pp(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> {
    let norm = isect.normal();
    loop {
        let new_ray_dir = random_vector_in_hemisphere(&norm, rng);
        if rng.gen::<f32>() < new_ray_dir.dot(&norm) {
            return Some(Ray{
                origin: isect.hit_pos() + norm.smul(EPSILON),
                dir: new_ray_dir
            })
        }
    }
}

fn flip_vector_to_hemisphere(flipee: &Vec3, norm: &Vec3) -> Vec3 {
    if flipee.dot(norm) > 0. {
        *flipee
    } else {
        -*flipee
    }
}

pub fn random_vector_in_hemisphere(norm: &Vec3, rng: &mut rand::ThreadRng) -> Vec3 {
    flip_vector_to_hemisphere(
        &random_normal(rng),
        norm
    )
}

pub fn random_normal(rng: &mut rand::ThreadRng) -> Vec3 {
    loop {
        let v = Vec3 {x: 1.-2.*rng.gen::<f32>(), y: 1.0-2.*rng.gen::<f32>(), z: 1.-2.*rng.gen::<f32>()};
        let len_sqr = v.x*v.x + v.y*v.y + v.z*v.z;
        if len_sqr <= 1. {
            return v.smul(1./len_sqr.sqrt());
        }
    }
}

fn new_random_ray_from_isect(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Ray {
    let last_isect_norm = isect.normal();
    let ray_start_pos = isect.hit_pos() + last_isect_norm.smul(EPSILON);
    let rand_dir = random_vector_in_hemisphere(&last_isect_norm, rng);
    Ray {origin: ray_start_pos, dir: rand_dir}
}

