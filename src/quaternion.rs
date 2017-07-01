use std::f32;
use std::ops::{Mul};

use vec3::Vec3;

#[derive(PartialEq, Clone, Copy)]
pub struct Quaternion {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32
}

impl Quaternion {
    pub fn vmul(&self, v: &Vec3) -> Vec3 {
        let qv = Vec3{x:self.b, y: self.c, z: self.d};
        let t = qv.cross(v).smul(2.);
        *v + t.smul(self.a) + qv.cross(&t)
    }
    pub fn from_axis_angle(axis: &Vec3, angle: f32) -> Quaternion {
        let s = (angle*0.5).sin();
        Quaternion {
            a: (angle*0.5).cos(),
            b: axis.x*s,
            c: axis.y*s,
            d: axis.z*s
        }
    }
}

impl Default for Quaternion {
    fn default() -> Quaternion {
        Quaternion{a: 1., b: 0., c: 0., d: 0.}
    }
}

impl Mul for Quaternion {
    type Output = Quaternion;

    fn mul(self, rhs: Quaternion) -> Quaternion {
        Quaternion{
            a: self.a*rhs.a - self.b*rhs.b - self.c*rhs.c - self.d*rhs.d,
            b: self.a*rhs.b + self.b*rhs.a - self.c*rhs.d + self.d*rhs.c,
            c: self.a*rhs.c + self.b*rhs.d + self.c*rhs.a - self.d*rhs.b,
            d: self.a*rhs.d - self.b*rhs.c + self.c*rhs.b + self.d*rhs.a
        }
    }
}
