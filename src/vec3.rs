use std::f32;
use std::ops::{Add, Sub, Neg};

#[derive(Debug, Copy, Clone, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl PartialEq for Vec3 {
    fn eq(&self, other: &Vec3) -> bool {
        return self.x==other.x && self.y==other.y && self.z==other.z
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3 { x: -self.x, y: -self.y, z: -self.z }
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Vec3 {
        Vec3 { x: self.x+other.x, y: self.y+other.y, z: self.z+other.z }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 { x: self.x-other.x, y: self.y-other.y, z: self.z-other.z }
    }
}

impl Vec3 {
    pub fn length(&self) -> f32 { f32::sqrt(self.dot(self)) }
    pub fn dot(&self, other: &Vec3) -> f32 { self.x*other.x + self.y*other.y + self.z*other.z }
    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3 { x: self.y*other.z - self.z*other.y,
               y: self.z*other.x - self.x*other.z,
               z: self.x*other.y - self.y*other.x }
    }
    pub fn smul(&self, s: f32) -> Vec3 {
        Vec3 { x: self.x*s, y: self.y*s, z: self.z*s }
    }
    pub fn normal(&self) -> Vec3 {
        self.smul(1.0 / self.length())
    }
}
