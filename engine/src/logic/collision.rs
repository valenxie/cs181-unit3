use core::f32;

use crate::graphics::animation::AnimationState;
use crate::logic::{
    geom::*,
    state::{EntityAnims, EntityState},
    types::*,
};
use cgmath::{num_traits::Pow, vec3, Vector3};

const SAMPLE_DENSITY: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct Contact<T: Copy> {
    pub a: T,
    pub b: T,
    pub mtv: Vec3,
}
pub struct Contacts {
    pub wm: Vec<Contact<usize>>,
    pub mm: Vec<Contact<usize>>,
}

impl Contacts {
    pub fn new() -> Self {
        Self {
            wm: vec![],
            mm: vec![],
        }
    }
    fn sort(&mut self) {
        self.wm
            .sort_unstable_by(|a, b| b.mtv.magnitude2().partial_cmp(&a.mtv.magnitude2()).unwrap());
        self.mm
            .sort_unstable_by(|a, b| b.mtv.magnitude2().partial_cmp(&a.mtv.magnitude2()).unwrap());
    }
    fn clear(&mut self) {
        self.wm.clear();
        self.mm.clear();
    }
}

// return a unit vector pointing from marble 1 to marble 2, i.e. contact normal
fn direction(marble1: &Marble, marble2: &Marble) -> Vector3<f32> {
    let mut disp = vec3(
        marble2.body.c.x - marble1.body.c.x,
        marble2.body.c.y - marble1.body.c.y,
        marble2.body.c.z - marble1.body.c.z,
    );
    let coef = (disp.x * disp.x + disp.y * disp.y + disp.z * disp.z).sqrt();
    disp * coef
}

// half the sum of momentum, abs value
fn avg_momentum(marble1: &Marble, marble2: &Marble, direction: Vector3<f32>) -> f32 {
    //let mut direction = direction(marble1, marble2);
    //marble1 velocity along direction of direction:
    let mut v1 = direction.x * marble1.velocity.x
        + direction.y * marble1.velocity.y
        + direction.y * marble1.velocity.y;
    v1 /= (direction.x.powf(2.0) + direction.y.powf(2.0) + direction.z.pow(2.0)).sqrt();
    //marble1 velocity along direction of direction:
    let mut v2 = direction.x * marble2.velocity.x
        + direction.y * marble2.velocity.y
        + direction.y * marble2.velocity.y;
    v2 /= (direction.x.powf(2.0) + direction.y.powf(2.0) + direction.z.pow(2.0)).sqrt();
    //safe to say that v2 is negative
    let sum_momentum =
        marble1.mass(SAMPLE_DENSITY) * v1.abs() + marble2.mass(SAMPLE_DENSITY) * v2.abs();
    sum_momentum * 0.5
}

pub fn update(walls: &[Wall], marbles: &mut [Marble], contacts: &mut Contacts) {
    contacts.clear();
    gather_contacts(walls, marbles, contacts);
    restitute(walls, marbles, contacts);
}

fn gather_contacts(statics: &[Wall], dynamics: &[Marble], into: &mut Contacts) {
    // collide mobiles against mobiles
    for (ai, a) in dynamics.iter().enumerate() {
        for (bi, b) in dynamics[(ai + 1)..].iter().enumerate() {
            let bi = ai + 1 + bi;
            if let Some(disp) = disp_sphere_sphere(&a.body, &b.body) {
                into.mm.push(Contact {
                    a: ai,
                    b: bi,
                    mtv: disp,
                });
            }
        }
    }
    // collide mobiles against walls
    todo!();
    /*
    for (bi, b) in statics.iter().enumerate() {
        for (ai, a) in dynamics.iter().enumerate() {
            if let Some(disp) = disp_sphere_plane(&a.body, &b.body) {
                into.wm.push(Contact {
                    a: ai,
                    b: bi,
                    mtv: disp,
                });
            }
        }
    }*/
}

fn restitute(walls: &[Wall], marbles: &mut [Marble], contacts: &mut Contacts) {
    contacts.sort();
    // Lots of marbles on the floor...
    for c in contacts.wm.iter() {
        let a = c.a;
        let b = c.b;
        // Are they still touching?  This way we don't need to track disps or anything
        // at the expense of some extra collision checks
        todo!();
        /*
        if let Some(disp) = disp_sphere_plane(&marbles[a].body, &walls[b].body) {
            // We can imagine we're instantaneously applying a
            // velocity change to pop the object just above the floor.
            marbles[a].body.c += disp;
            // It feels a little weird to be adding displacement (in
            // units) to velocity (in units/frame), but we'll roll
            // with it.  We're not exactly modeling a normal force
            // here but it's something like that.
            marbles[a].velocity += disp;
        }*/
    }
    // That can bump into each other in perfectly elastic collisions!
    for c in contacts.mm.iter() {
        let a = c.a;
        let b = c.b;
        // Just split the difference.  In crowded situations this will
        // cause issues, but those will always be hard to solve with
        // this kind of technique.
        if let Some(disp) = disp_sphere_sphere(&marbles[a].body, &marbles[b].body) {
            let direction = direction(&marbles[a], &marbles[b]);
            let avg_momentum = avg_momentum(&marbles[a], &marbles[b], direction);
            let impulse = avg_momentum * direction;
            marbles[a].velocity -= impulse;
            marbles[b].velocity += impulse;
        }
    }
}
