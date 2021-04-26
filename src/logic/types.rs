use std::f32::consts::PI;

use crate::logic::geom::*;
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u16,
    pub h: u16,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Marble {
    pub body: Sphere,
    pub velocity: Vec3,
}

impl Marble {
    pub fn mass(&self,density: f32)->f32{
        self.body.r.powi(3) * PI * 4.0 * density / 3.0
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wall {
    pub body: Block,
    pub distructable: bool,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Vec2i(pub i32, pub i32);

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

// Feel free to add impl blocks with convenience functions
