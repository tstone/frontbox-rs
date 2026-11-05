use crate::*;

/// Keeps a single value for a set amount, then completes. Useful as a pause inside a [`Sequence`], and works with any
/// value, including ones that can't be interpolated.
///
/// ```rust
/// # use std::time::Duration;
/// # use frontbox_animation::*;
/// let mut seq = Sequence::new(
///   vec![
///     Hold::boxed(Duration::from_secs(2), 0),
///     Tween::boxed(Duration::from_secs(1), Curve::Linear, vec![0, 100], Cycle::Once),
///   ],
///   Cycle::Once,
/// );
///
/// seq.accumulate(Duration::from_millis(2500));
/// assert_eq!(seq.sample(), 50);
/// ```
#[derive(Clone)]
pub struct Hold<A, T> {
  /// How long the value is held
  pub target: A,
  pub value: T,
  active: bool,
  current: A,
  complete: bool,
}

impl<A, T> Hold<A, T>
where
  A: Tweenable,
{
  pub fn new(target: A, value: T) -> Self {
    Self {
      target,
      value,
      active: true,
      current: A::default(),
      complete: false,
    }
  }

  pub fn boxed(target: A, value: T) -> Box<Self> {
    Box::new(Self::new(target, value))
  }

  /// Hold starts in a playing state by default. Use this to chain it to prevent that.
  pub fn stopped(mut self) -> Self {
    self.active = false;
    self
  }
}

impl<A, T> Accumulator<A> for Hold<A, T>
where
  A: Tweenable,
  T: Clone,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    let mut result = AccumulationResult::default();
    if !self.active || self.complete {
      return result;
    }

    self.current = self.current + delta;
    if self.current >= self.target {
      result.completed_cycle = true;
      // a zero (or negative) length hold passes the whole delta on
      result.remainder = if self.target > A::default() {
        self.current - self.target
      } else {
        delta
      };
      self.current = A::default();
      self.complete = true;
    }

    result
  }

  fn force(&mut self, current: A) {
    self.current = current;
    self.complete = false;
  }

  fn reset(&mut self) {
    self.current = A::default();
    self.complete = false;
  }

  fn is_complete(&self) -> bool {
    self.complete
  }
}

impl<A, T> Animation<A, T> for Hold<A, T>
where
  A: Tweenable + 'static,
  T: Clone + Send + Sync + 'static,
{
  fn sample(&self) -> T {
    self.value.clone()
  }

  fn play(&mut self) {
    self.active = true;
  }

  fn stop(&mut self) {
    self.active = false;
  }

  fn active(&self) -> bool {
    self.active
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_holds_value_until_complete() {
    let mut hold = Hold::new(1.0f32, "red");
    assert_eq!(hold.sample(), "red");

    let result = hold.accumulate(0.5);
    assert!(!result.completed_cycle);
    assert!(!hold.is_complete());

    let result = hold.accumulate(0.75);
    assert!(result.completed_cycle);
    assert_eq!(result.remainder, 0.25);
    assert!(hold.is_complete());
    assert_eq!(hold.sample(), "red");
  }

  #[test]
  fn test_zero_length_completes_on_first_advance() {
    let mut hold = Hold::new(0.0f32, 1);
    assert!(!hold.is_complete());

    let result = hold.accumulate(0.5);
    assert!(hold.is_complete());
    assert_eq!(result.remainder, 0.5);
  }

  #[test]
  fn test_stopped_waits_for_play() {
    let mut hold = Hold::new(1.0f32, 1).stopped();
    hold.accumulate(2.0);
    assert!(!hold.is_complete());

    hold.play();
    hold.accumulate(1.0);
    assert!(hold.is_complete());
  }

  #[test]
  fn test_reset_and_force() {
    let mut hold = Hold::new(1.0f32, 1);
    hold.accumulate(1.0);
    hold.reset();
    assert!(!hold.is_complete());

    hold.force(0.75);
    let result = hold.accumulate(0.5);
    assert!(hold.is_complete());
    assert_eq!(result.remainder, 0.25);
  }

  #[test]
  fn test_pauses_inside_a_sequence() {
    let mut seq: Sequence<f32, f32> = Sequence::new(
      vec![
        Tween::boxed(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once),
        Hold::boxed(1.0, 10.0),
        Tween::boxed(1.0, Curve::Linear, vec![10.0, 0.0], Cycle::Once),
      ],
      Cycle::Forever,
    );

    seq.accumulate(1.5);
    assert_eq!(seq.sample(), 10.0);

    seq.accumulate(1.0);
    assert_eq!(seq.sample(), 5.0);

    // the hold plays again on the next cycle
    seq.accumulate(2.0);
    assert_eq!(seq.sample(), 10.0);
  }
}
