extern crate rand;
use std::default::Default;
use vec3::Vec3;
use color3f::Color3f;

#[derive(Clone,Copy,Default)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3
}

#[derive(Clone)]
pub enum Primitive {
    Sphere(Vec3, f32),
    Triangle(Vec3, Vec3, Vec3)
}

impl Default for Primitive {
    fn default() -> Primitive {
        Primitive::Sphere(Vec3::default(), 0.)
    }
}

pub type IsectProg = fn(&RayIsect, &mut rand::ThreadRng) -> Option<Ray>;

pub struct Material {
    pub emissive: Color3f,
    pub diffuse: Color3f,
    pub isect_prog: IsectProg
}

impl Clone for Material {
    fn clone(&self) -> Material {
        Material {
            emissive: self.emissive,
            diffuse: self.diffuse,
            isect_prog: self.isect_prog
        }
    }
}

fn default_isect_prog(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> { None }
impl Default for Material {
    fn default() -> Material {
        Material {
            emissive: Color3f::default(),
            diffuse: Color3f::default(),
            isect_prog: default_isect_prog
        }
    }
}

#[derive(Clone,Default)]
pub struct SceneObj {
    pub prim: Primitive,
    pub mat: Material
}

#[derive(Clone,Copy)]
pub enum IsectFrom { Outside, Inside }

#[derive(Clone,Copy)]
pub struct RayIsect<'a> {
    pub ray: Ray,
    pub dist: f32,
    pub from: IsectFrom,
    pub scene_obj: &'a SceneObj
}

pub const MAX_BOUNCES: usize = 6;

pub struct Path<'a> {
    pub num_bounces: i32,
    pub isects: [RayIsect<'a>; MAX_BOUNCES]
}

#[test]
fn test_vec3() {
    let x = Vec3 {x: 3.0, y: 4.0, z: 0.0};
    let ix = Vec3 { x: 1.0, y: 0.0, z: 0.0};
    let iy = Vec3 { x: 0.0, y: 1.0, z: 0.0};
    assert_eq!(x.length(), 5.0);
    assert_eq!(x.dot(&x), 25.0);
    assert_eq!(ix + iy, Vec3 { x: 1.0, y: 1.0, z: 0.0});
    assert_eq!(ix - iy, Vec3 { x: 1.0, y: -1.0, z: 0.0});
    assert_eq!(ix.cross(&iy), Vec3 { x: 0.0, y: 0.0, z: 1.0});
    assert_eq!(x.smul(10.0), Vec3 { x: 30.0, y: 40.0, z: 0.0});
    assert_eq!(x.normal().length(), 1.0);
}

#[test]
fn test_color3f() {
    let c = Color3f {r: 1.0, g: 0.5, b: 0.25};
    assert_eq!(c+c, Color3f { r: 2.0, g: 1.0, b: 0.5});
}
