use dyn_clone::DynClone;

use crate::animation::{AccumulationResult, Accumulator};

pub trait DynModulation<S, A>: DynClone {
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A>;
  fn force(&mut self, current: A);
  fn reset(&mut self);
  fn apply(&mut self, delta: A, target: &mut S);
  fn is_complete(&self) -> bool;
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
