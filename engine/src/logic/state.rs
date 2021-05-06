use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::logic::types::{Rect, Vec2i};
use crate::{
    audio::audio::SoundChannels,
    graphics::{
        animation::{Animation, AnimationState},
        texture::CpuTexture,
    },
};
use rand::StdRng;

#[derive(Clone)]
pub enum StateType {
    Menu(GameState),
    Playing(GameState),
    GameOver(GameState),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntityType {
    Player,
    Enemy,
}

pub type Level = (
    Vec<(EntityType, i32, i32)>,
    Vec<Vec<usize>>,
    Vec<usize>,
    Vec<usize>,
);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Inputs {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub space: bool,
    pub esc: bool,
}

impl Inputs {
    pub fn new() -> Inputs {
        Inputs {
            up: false,
            down: false,
            left: false,
            right: false,
            space: false,
            esc: false,
        }
    }
}

// Frame Numbers
const STAND_FRAME: u16 = 0;
const RUN_START: u16 = 1;
const MID_RUN: u16 = 14;
const RUN_END: u16 = 22;
const JUMP_START: u16 = 24;
const JUMP_END: u16 = 29;
const FALL_START: u16 = 30;
const FALL_END: u16 = 36;
const LAND_START: u16 = 37;
const LAND_END: u16 = 47;
const FRAME_LEN: usize = 3;

#[derive(Clone, Eq, PartialEq)]
pub enum EntityState {
    Standing,
    StartRun,
    Running,
    Jumping,
    Falling,
    Landing,
}

#[derive(Clone)]
pub struct EntityAnims {
    pub standing: Rc<Animation>,
    pub start_run: Rc<Animation>,
    pub running: Rc<Animation>,
    pub jumping: Rc<Animation>,
    pub falling: Rc<Animation>,
    pub landing: Rc<Animation>,
}

impl EntityAnims {
    pub fn new() -> EntityAnims {
        EntityAnims {
            standing: Rc::new(gen_frames(STAND_FRAME, RUN_START, FRAME_LEN, true)),
            start_run: Rc::new(gen_frames(RUN_START, MID_RUN, FRAME_LEN, false)),
            running: Rc::new(gen_frames(MID_RUN, RUN_END, FRAME_LEN, true)),
            jumping: Rc::new(gen_frames(JUMP_START, JUMP_END, FRAME_LEN, false)),
            falling: Rc::new(gen_frames(FALL_START, FALL_END, FRAME_LEN, true)),
            landing: Rc::new(gen_frames(LAND_START, LAND_END, FRAME_LEN, false)),
        }
    }
}

fn gen_frames(start: u16, end: u16, len: usize, looping: bool) -> Animation {
    let mut anim = Vec::new();
    for frame in start..end {
        let w = 25;
        let h = 16;
        let x = (frame * w) % 200;
        let y = (frame / 8) * h;
        anim.push((
            Rect {
                w,
                h,
                x: x as i32,
                y: y as i32,
            },
            len,
        ));
    }
    Animation::new(anim, looping)
}

#[derive(Clone)]
pub struct GameState {
    // Every entity has a position, a size, a texture, and animation state.
    // Assume entity 0 is the player
    pub types: Vec<EntityType>,
    pub ent_states: Vec<EntityState>,
    pub positions: Vec<Vec2i>,
    pub velocities: Vec<Vec2i>,
    pub sizes: Vec<(usize, usize)>,
    pub textures: Vec<Rc<CpuTexture>>,
    pub anim_state: Vec<AnimationState>,
    // Current level
    pub level: usize,
    // Camera position
    pub camera: Vec2i,
    pub inputs: Inputs,
    pub menu_entry: usize,
    pub entity_anims: EntityAnims,
    pub sound_channels: Arc<Mutex<SoundChannels>>,
    pub score: usize,
    pub seed: u64,
    pub rng: StdRng,
}
