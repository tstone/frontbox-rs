use crate::animation::*;

#[derive(Clone, Default)]
pub struct MultiModulator<S, A> {
  modulators: Vec<Box<dyn DynModulation<S, A>>>,
}

impl<S, A> MultiModulator<S, A> {
  pub fn new(modulators: Vec<Box<dyn DynModulation<S, A>>>) -> Self {
    Self { modulators }
  }

  pub fn add(&mut self, modulation: impl DynModulation<S, A> + 'static) {
    self.modulators.push(Box::new(modulation));
  }

  /// Advances every modulator by `delta`, then applies all of them to `target`.
  pub fn apply(&mut self, delta: A, target: &mut S)
  where
    A: Clone,
  {
    for m in &mut self.modulators {
      m.accumulate(delta.clone()); // fan-out; per-modulator remainders discarded
    }
    for m in &mut self.modulators {
      m.apply(delta.clone(), target);
    }
  }
}
