use std::sync::Arc;

use crate::*;

/// Something which advances by `delta` (type `A`) and applies the result to a target (type `S`)
pub trait Modulation<S, A> {
  fn apply(&mut self, delta: A, target: &mut S);
}

pub type ModulationSetter<S, T> = dyn Fn(&mut S, T) + Send + Sync + 'static;

/// A modulator combines an accumulator with a setter, mutating a value over time. Like accumulators, these can be used direct if needed, but are generally used through higher level constructors (like LedProgram1ds).
///
/// ```rust
/// # use std::time::Duration;
/// # use frontbox_animation::*;
/// # let anim = Tween::new(Duration::from_secs(1), Curve::Linear, vec![0u8, 255], Cycle::Once);
/// struct Light { brightness: u8 }
///
/// let mut modulator = Modulator::new(
///   anim,
///   |target: &mut Light, brightness| target.brightness = brightness
/// );
///
/// let mut light = Light { brightness: 0 };
/// modulator.apply(Duration::from_millis(500), &mut light);
/// ```
#[derive(Clone)]
pub struct Modulator<S, T, A> {
  setter: Arc<ModulationSetter<S, T>>,
  animation: Box<dyn Animation<A, T>>,
}

impl<S, T, A> Modulator<S, T, A> {
  pub fn new(
    animation: impl Animation<A, T> + 'static,
    setter: impl Fn(&mut S, T) + Send + Sync + 'static,
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

impl<S, T, A> Modulation<S, A> for Modulator<S, T, A> {
  /// Accumulates delta and applies to target
  fn apply(&mut self, delta: A, target: &mut S) {
    self.animation.accumulate(delta);
    (self.setter)(target, self.animation.sample());
  }
}

impl<S, T, A> Accumulator<A> for Modulator<S, T, A>
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
}

impl<S, T, A> DynModulation<S, A> for Modulator<S, T, A>
where
  S: Clone + 'static,
  T: Clone + 'static,
  A: Clone + 'static,
{
  fn play(&mut self) {
    self.animation.play();
  }

  fn stop(&mut self) {
    self.animation.stop();
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
  fn test_apply_sets_target() {
    let mut modulator = ramp();
    let mut value = -1.0;

    modulator.apply(0.25, &mut value);
    assert_eq!(value, 2.5);

    modulator.apply(0.25, &mut value);
    assert_eq!(value, 5.0);
  }

  #[test]
  fn test_stopped_still_sets_current_value() {
    let mut modulator = ramp();
    let mut value = 0.0;

    modulator.apply(0.5, &mut value);
    modulator.stop();
    value = -1.0;
    modulator.apply(0.25, &mut value);
    assert_eq!(value, 5.0);

    modulator.play();
    modulator.apply(0.25, &mut value);
    assert_eq!(value, 7.5);
  }

  #[test]
  fn test_accumulator_forwards_to_animation() {
    let mut modulator = ramp();
    let mut value = 0.0;

    modulator.force(0.75);
    modulator.apply(0.0, &mut value);
    assert_eq!(value, 7.5);

    let result = modulator.accumulate(0.5);
    assert!(result.completed_cycle);
    assert_eq!(result.remainder, 0.25);
    assert!(modulator.is_complete());

    modulator.reset();
    assert!(!modulator.is_complete());
    modulator.apply(0.0, &mut value);
    assert_eq!(value, 0.0);
  }
}
