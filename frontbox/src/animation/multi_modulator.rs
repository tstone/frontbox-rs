use crate::animation::*;

#[derive(Clone, Default)]
pub struct MultiModulator<S, A> {
  active: bool,
  modulators: Vec<Box<dyn DynModulation<S, A> + Send + Sync>>,
}

impl<S, A> MultiModulator<S, A> {
  pub fn new(modulators: Vec<Box<dyn DynModulation<S, A> + Send + Sync>>, active: bool) -> Self {
    Self { modulators, active }
  }

  pub fn playing(modulators: Vec<Box<dyn DynModulation<S, A> + Send + Sync>>) -> Self {
    Self::new(modulators, true)
  }

  pub fn stopped(modulators: Vec<Box<dyn DynModulation<S, A> + Send + Sync>>) -> Self {
    Self::new(modulators, false)
  }

  pub fn add(&mut self, modulation: impl DynModulation<S, A> + Send + Sync + 'static) {
    self.modulators.push(Box::new(modulation));
  }

  pub fn play(&mut self) {
    self.active = true;
    for modulator in &mut self.modulators {
      modulator.play();
    }
  }

  pub fn stop(&mut self) {
    self.active = false;
  }

  pub fn active(&self) -> bool {
    self.active
  }

  pub fn reset(&mut self) {
    for modulator in &mut self.modulators {
      modulator.reset();
    }
  }

  pub fn is_complete(&self) -> bool {
    self.modulators.iter().all(|m| m.is_complete())
  }

  /// Advances every modulator by `delta`, then applies all of them to `target`.
  pub fn apply(&mut self, delta: A, target: &mut S)
  where
    A: Clone,
  {
    if self.active {
      for m in &mut self.modulators {
        m.accumulate(delta.clone()); // fan-out; per-modulator remainders discarded
      }
      for m in &mut self.modulators {
        m.apply(delta.clone(), target);
      }
    }
  }
}
