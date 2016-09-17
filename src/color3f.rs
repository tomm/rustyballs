use std::ops::{Add, Mul, AddAssign};


#[derive(Debug, Copy, Clone, Default)]
pub struct Color3f {
    pub r: f32,
    pub g: f32,
    pub b: f32
}

impl Color3f {
    pub fn smul(self, num: f32) -> Color3f {
        Color3f {r: self.r*num, g: self.g*num, b: self.b*num}
    }
    pub fn black() -> Color3f { Color3f{r:0.,g:0.,b:0.} }
    pub fn max_channel(&self) -> f32 {
        if self.r > self.g {
            if self.r > self.b {self.r} else {self.b}
        } else {
            if self.g > self.b {self.g} else {self.b}
        }
    }
}

impl Add for Color3f {
    type Output = Color3f;

    fn add(self, other: Color3f) -> Color3f {
        Color3f { r: self.r+other.r, g: self.g+other.g, b: self.b+other.b }
    }
}

impl AddAssign for Color3f {
    fn add_assign(&mut self, rhs: Color3f) {
        self.r = self.r + rhs.r;
        self.g = self.g + rhs.g;
        self.b = self.b + rhs.b;
    }
}

impl Mul for Color3f {
    type Output = Color3f;

    fn mul(self, other: Color3f) -> Color3f {
        Color3f { r: self.r*other.r, g: self.g*other.g, b: self.b*other.b }
    }
}

impl PartialEq for Color3f {
    fn eq(&self, other: &Color3f) -> bool {
        return self.r==other.r && self.g==other.g && self.b==other.b
    }
}
