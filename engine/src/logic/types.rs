use std::f32::consts::PI;

use super::geom::*;
use crate::graphics::gpu::InstanceRaw;

const DT: f32 = 1.0 / 60.0;

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
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from_translation(self.body.c.to_vec()) * Mat4::from_scale(self.body.r))
                .into(),
        }
    }
    fn update(&mut self, g: f32) {
        self.velocity += Vec3::new(0.0, -g * (1.0 + (self.body.r - 0.1) * 0.5), 0.0) * DT;
        self.body.c += self.velocity * DT;
    }

    pub fn mass(&self, density: f32) -> f32 {
        //V=4/3pi*r^3
        let volume = (4.0 / 3.0) * PI * (self.body.r * self.body.r * self.body.r);
        volume * density
    }
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wall {
    pub body: Plane,
    pub distructable: bool,
}

impl Wall {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from(cgmath::Quaternion::between_vectors(
                Vec3::new(0.0, 1.0, 0.0),
                self.body.n,
            )) * Mat4::from_translation(Vec3::new(0.0, -0.025, 0.0))
                * Mat4::from_nonuniform_scale(0.5, 0.05, 0.5))
            .into(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Vec2i(pub i32, pub i32);

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

// Feel free to add impl blocks with convenience functions
