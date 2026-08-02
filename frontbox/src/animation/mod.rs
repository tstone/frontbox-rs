mod accumulator;
mod curve;
mod lerp;
mod modulation;
mod modulator;
mod multi_modulator;
mod sequence;
mod single_frame_anim;
mod tween;
mod tweenable;

pub use accumulator::*;
pub use curve::*;
pub use lerp::*;
pub use modulation::*;
pub use modulator::*;
pub use multi_modulator::*;
pub use sequence::*;
pub use single_frame_anim::*;
pub use tween::*;
pub use tweenable::*;

/// Describes any value that can be changed over time. More specifically, an animation is a Tickable (something which can be marched forward with time) that returns a value.
pub trait Animation<Acc, Val>: Accumulator<Acc> {
  fn sample(&self) -> Val;

  fn play(&mut self);
  fn pause(&mut self);

  fn stop(&mut self) {
    self.reset();
    self.pause();
  }
}

dyn_clone::clone_trait_object!(<A, T> Animation<A, T>);
