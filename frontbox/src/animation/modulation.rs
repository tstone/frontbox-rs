use dyn_clone::DynClone;

use crate::animation::{AccumulationResult, Accumulator, Modulation};

/// Object-safe, cloneable extension of [`Modulation`], allowing modulations to be stored as `Box<dyn DynModulation>`.
pub trait DynModulation<S, A>: Modulation<S, A> + DynClone {
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A>;
  fn force(&mut self, current: A);
  fn reset(&mut self);
  fn is_complete(&self) -> bool;
  fn play(&mut self);
  fn stop(&mut self);
}

dyn_clone::clone_trait_object!(<S, A> DynModulation<S, A>);

impl<S, A> Accumulator<A> for dyn DynModulation<S, A> {
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    self.accumulate(delta)
  }

  fn is_complete(&self) -> bool {
    self.is_complete()
  }

  fn reset(&mut self) {
    self.reset();
  }

  fn force(&mut self, current: A) {
    self.force(current);
  }
}
