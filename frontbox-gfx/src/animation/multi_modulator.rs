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
    for modulator in &mut self.modulators {
      modulator.stop();
    }
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
        m.apply(delta.clone(), target);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn ramp() -> Modulator<f32, f32, f32> {
    Modulator::new(
      Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once),
      |target: &mut f32, value| *target = value,
    )
  }

  #[test]
  fn test_apply_advances_children() {
    let mut multi = MultiModulator::playing(vec![Box::new(ramp())]);
    let mut value = 0.0;

    multi.apply(0.5, &mut value);
    assert_eq!(value, 5.0);
  }

  #[test]
  fn test_stop_stops_children() {
    let mut multi = MultiModulator::playing(vec![Box::new(ramp())]);
    let mut value = 0.0;

    multi.stop();
    multi.apply(0.5, &mut value);
    assert_eq!(value, 0.0);

    // the children are stopped too, so advancing one directly does nothing
    for modulator in &mut multi.modulators {
      modulator.accumulate(0.5);
    }

    multi.play();
    multi.apply(0.25, &mut value);
    assert_eq!(value, 2.5);
  }

  #[test]
  fn test_stopped_does_nothing_until_played() {
    let mut multi = MultiModulator::stopped(vec![Box::new(ramp())]);
    let mut value = -1.0;

    multi.apply(0.5, &mut value);
    assert!(!multi.active());
    assert_eq!(value, -1.0);

    multi.play();
    multi.apply(0.5, &mut value);
    assert_eq!(value, 5.0);
  }

  #[test]
  fn test_applies_every_modulator_in_order() {
    let mut multi = MultiModulator::playing(vec![Box::new(ramp())]);
    // a second modulator that doubles whatever the first one set
    multi.add(Modulator::new(
      Tween::new(1.0, Curve::Constant(0.0), vec![0.0, 0.0], Cycle::Once),
      |target: &mut f32, _: f32| *target *= 2.0,
    ));
    let mut value = 0.0;

    multi.apply(0.5, &mut value);
    assert_eq!(value, 10.0);
  }

  #[test]
  fn test_complete_when_every_modulator_is() {
    let mut multi = MultiModulator::playing(vec![Box::new(ramp())]);
    multi.add(Modulator::new(
      Tween::new(2.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once),
      |_: &mut f32, _: f32| {},
    ));
    let mut value = 0.0;

    multi.apply(1.0, &mut value);
    assert!(!multi.is_complete());

    multi.apply(1.0, &mut value);
    assert!(multi.is_complete());

    multi.reset();
    assert!(!multi.is_complete());
    multi.apply(0.5, &mut value);
    assert_eq!(value, 5.0);
  }
}
