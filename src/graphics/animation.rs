use crate::logic::types::Rect;
use std::rc::Rc;

#[derive(Debug)]
pub struct Animation {
    // Do this for the exercise today!
    // You'll want to know the frames involved and the timing for each frame
    // But then there's also dynamic data, which might live in this struct or might live somewhere else
    // An Animation/AnimationState split could be fine, if AnimationState holds the start time and the present frame (or just the start time) and possibly a reference to the Animation
    // but there are lots of designs that will work!
    frames: Vec<(Rect, usize)>,
    looping: bool,
}

impl Animation {
    pub fn new(frames: impl IntoIterator<Item = (Rect, usize)>, looping: bool) -> Self {
        Self {
            frames: frames.into_iter().collect(),
            looping,
        }
    }
    pub fn freeze(r: Rect) -> Self {
        Self::new(vec![(r, 0)], false)
    }
    // Should hold some data...
    // Be used to decide what frame to use...
    // And sprites can be updated based on that information.
    // Or we could give sprites an =animation= field instead of a =frame=!
    // Could have a query function like current_frame(&self, start_time:usize, now:usize, speedup_factor:usize)
    // Or could be ticked in-place
    pub fn start(self: &Rc<Animation>) -> AnimationState {
        AnimationState {
            animation: Rc::clone(self),
            time: 0,
        }
    }
    pub fn duration(&self) -> usize {
        self.frames.iter().map(|(_, t)| t).sum()
    }
}

#[derive(Clone, Debug)]
pub struct AnimationState {
    animation: Rc<Animation>,
    time: usize,
}
impl AnimationState {
    pub fn frame(&self) -> Rect {
        let mut t = 0;
        for (cr, ct) in self.animation.frames.iter() {
            t += ct;
            if t >= self.time {
                return *cr;
            }
        }
        panic!(
            "Animation frame not found for t={}, anim={:?}",
            self.time, self.animation
        );
    }
    pub fn done(&self) -> bool {
        self.time >= self.animation.duration()
    }
    pub fn tick(&mut self) {
        let dur = self.animation.duration();
        self.time = if self.animation.looping {
            (self.time + 1) % dur
        } else {
            (self.time + 1).min(dur)
        };
    }
    pub fn play(&mut self, anim: &Rc<Animation>, force: bool) {
        if self.done() || force {
            *self = anim.start();
        }
    }
}
