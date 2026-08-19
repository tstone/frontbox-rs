use std::sync::Arc;

use crate::animation::*;

/// A modulator combines an accumulator with a setter, mutating a value over time. Like accumulators, these can be used direct if needed, but are generally used through higher level constructors (like LedPrograms).
///
/// ```rust
/// let modulator = Modulator::new(
///   self.anim,
///   |value| {  }
/// )
/// ```
pub trait Modulation<A, S> {
  fn apply(&mut self, delta: A, target: &mut S);
}

pub type ModulationSetter<S, T> = dyn Fn(&mut S, T) + 'static;

#[derive(Clone)]
pub struct Modulator<S, T, A> {
  setter: Arc<ModulationSetter<S, T>>,
  animation: Box<dyn Animation<A, T>>,
}

impl<S, T, A> Modulator<S, T, A> {
  pub fn new(
    animation: impl Animation<A, T> + 'static,
    setter: impl Fn(&mut S, T) + 'static,
  ) -> Self {
    Modulator {
      setter: Arc::new(setter),
      animation: Box::new(animation),
    }
  }

  pub fn play(&mut self) {
    self.animation.play();
  }

  pub fn stop(&mut self) {
    self.animation.stop();
  }
}

impl<S, T, A> Modulation<A, S> for Modulator<S, T, A> {
  /// Accumulates delta and applies to target
  fn apply(&mut self, delta: A, target: &mut S) {
    self.animation.accumulate(delta);
    (self.setter)(target, self.animation.sample());
  }
}

impl<S, T, A> DynModulation<S, A> for Modulator<S, T, A>
where
  S: Clone + 'static,
  T: Clone + 'static,
  A: Clone + 'static,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    self.animation.accumulate(delta)
  }

  fn force(&mut self, current: A) {
    self.animation.force(current);
  }

  fn reset(&mut self) {
    self.animation.reset();
  }

  fn is_complete(&self) -> bool {
    self.animation.is_complete()
  }

  fn apply(&mut self, delta: A, target: &mut S) {
    Modulation::apply(self, delta, target)
  }
}
