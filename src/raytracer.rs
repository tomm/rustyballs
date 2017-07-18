extern crate rand;
use rand::Rng; // why did i need this for rng.gen?
use std::default::Default;
use vec3::Vec3;
use color3f::Color3f;
use quaternion::Quaternion;
use shaders;

pub const EPSILON: f32 = 0.0001;

#[derive(Clone,Copy,Default)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3
}

pub struct RenderConfig {
    pub threads: usize,
    pub samples_per_first_isect: u32,
    pub image_size: (u32, u32),
    pub preview_hdr_gamma: f32
}
impl Default for RenderConfig {
    fn default() -> RenderConfig {
        RenderConfig{
            threads: 8,
            samples_per_first_isect: 20,
            image_size: (512, 512),
            preview_hdr_gamma: 100.0
        }
    }
}

#[derive(Clone)]
pub enum Primitive {
    Sphere(Vec3, f32),
    Triangle(Vec3, Vec3, Vec3),
    Plane(Vec3, Vec3),  // (origin, normal)
    ScatterEvent
}

impl Default for Primitive {
    fn default() -> Primitive {
        Primitive::Sphere(Vec3::default(), 0.)
    }
}

pub enum VacuumAction {
    Continue,
    Scatter(RayIsect<'static>)
}

#[derive(Clone,Copy)]
pub struct ColorProgramResult {
    pub transmissive: Color3f,
    pub emissive: Color3f
}

pub type PathProgram = fn(&RayIsect, &mut rand::ThreadRng) -> Option<Ray>;
pub type ColorProgram = fn(&RayIsect) -> ColorProgramResult;
pub type VacuumProgram = fn(&RayIsect, &mut rand::ThreadRng) -> VacuumAction;

pub struct Material {
    pub color_program: ColorProgram,
    pub path_program: PathProgram,
    pub vacuum_program: Option<VacuumProgram>
}

impl Clone for Material {
    fn clone(&self) -> Material {
        Material {
            path_program: self.path_program,
            color_program: self.color_program,
            vacuum_program: None
        }
    }
}

fn default_path_program(isect: &RayIsect, rng: &mut rand::ThreadRng) -> Option<Ray> { None }
fn default_color_program(isect: &RayIsect) -> ColorProgramResult {
    ColorProgramResult { emissive: Color3f::default(), transmissive: Color3f::default() }
}
impl Default for Material {
    fn default() -> Material {
        Material {
            color_program: default_color_program,
            path_program: default_path_program,
            vacuum_program: None
        }
    }
}

pub struct Scene {
    pub objs: Vec<SceneObj>
}

#[derive(Copy,Clone)]
pub struct Camera {
    pub position: Vec3,
    pub orientation: Quaternion
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

impl<'a> RayIsect<'a> {
    pub fn hit_pos(&self) -> Vec3 {
        self.ray.origin + self.ray.dir.smul(self.dist)
    }
    pub fn normal(&self) -> Vec3 {
        match self.scene_obj.prim {
            Primitive::Sphere(origin, _) => (self.hit_pos() - origin).normal(),
            Primitive::Triangle(v1, v2, v3) => (v2-v1).cross(&(v2-v3)).normal(),
            Primitive::Plane(_, normal) => normal,
            Primitive::ScatterEvent => -self.ray.dir.normal()
        }
    }
    pub fn new_random_ray(&self, rng: &mut rand::ThreadRng) -> Ray {
        let last_isect_norm = self.normal();
        let ray_start_pos = self.hit_pos() + last_isect_norm.smul(EPSILON);
        let rand_dir = shaders::random_vector_in_hemisphere(&last_isect_norm, rng);
        Ray {origin: ray_start_pos, dir: rand_dir}
    }
}

pub const MAX_BOUNCES: usize = 6;

pub struct Path<'a> {
    pub num_bounces: i32,
    pub isects: [RayIsect<'a>; MAX_BOUNCES],
    // cached first isect color result. we re-use first isect!
    pub first_isect_color: Option<ColorProgramResult>
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
