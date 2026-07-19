use std::cmp::max;
use std::fmt::Debug;
use std::ops::{AddAssign, SubAssign};

use crate::animation::*;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified quantity using a given curve
#[derive(Clone)]
pub struct Tween<A: Tweenable + Copy + Default + Debug, T: Lerp + Clone + Send + Sync> {
  // the amount which signals this animation is done
  pub target: A,
  current: A,
  pub curve: Curve,
  pub stops: Vec<T>,
  pub cycle: AnimationCycle,
  cycle_count: u32,
  current_stop_index: usize,
}

impl<A, T> Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: Tweenable + Copy + Default + AddAssign + SubAssign + PartialEq + Send + Sync + Debug,
{
  /// *target* - Accumulated value to get to, e.g. Duration::from_secs(1) = animation lasts for a minute
  pub fn new(target: A, curve: Curve, stops: Vec<T>, cycle: AnimationCycle) -> Self {
    assert!(stops.len() >= 2, "Tween requires at least 2 stops");

    Self {
      target: target.div_usize(stops.len() - 1),
      current: A::default(),
      curve,
      stops,
      cycle,
      cycle_count: 0,
      current_stop_index: 0,
    }
  }

  pub fn once(target: A, curve: Curve, stops: Vec<T>) -> Self {
    Self::new(target, curve, stops, AnimationCycle::Once)
  }

  pub fn forever(target: A, curve: Curve, stops: Vec<T>) -> Self {
    Self::new(target, curve, stops, AnimationCycle::Forever)
  }

  pub fn linear(target: A, stops: Vec<T>, cycle: AnimationCycle) -> Self {
    Self::new(target, Curve::Linear, stops, cycle)
  }

  pub fn ping_pong(target: A, curve: Curve, stops: Vec<T>, cycle: AnimationCycle) -> Sequence<A, T>
  where
    A: PartialOrd + 'static,
    T: 'static,
  {
    Sequence::new(
      vec![
        Tween::boxed(target, curve.clone(), stops.clone(), AnimationCycle::Once)
          as Box<dyn Animation<A, T>>,
        Tween::boxed(
          target,
          Curve::Reverse(Box::new(curve.clone())),
          stops.into_iter().rev().collect(),
          AnimationCycle::Once,
        ),
      ],
      cycle,
    )
  }

  pub fn boxed(target: A, curve: Curve, stops: Vec<T>, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self::new(target, curve, stops, cycle))
  }

  fn next_index(&self) -> usize {
    let idx = self.current_stop_index + 1;
    if idx == self.stops.len() {
      return 0;
    }
    return idx;
  }

  pub fn reverse(&mut self) {
    Tween {
      target: self.target,
      current: A::default(),
      curve: Curve::Reverse(Box::new(self.curve.clone())),
      stops: self.stops.clone().into_iter().rev().collect(),
      cycle: self.cycle.clone(),
      cycle_count: self.cycle_count,
      current_stop_index: self.current_stop_index,
    };
  }
}

impl<A, T> Accumulator<A> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: Tweenable
    + Copy
    + Default
    + AddAssign
    + SubAssign
    + PartialOrd
    + PartialEq
    + Send
    + Sync
    + Debug,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    let mut result = AccumulationResult {
      remainder: A::default(),
      completed_cycle: false,
    };

    if self.is_complete() {
      return AccumulationResult::default();
    }

    // Edge case: In Times & Forever modes the current stop is only reset if the accumulated value overshoots the target
    // This is so that in the case where the accumulation is exactly the target, the `sample()` method will return the
    // final stop value instead of jumping back to the first stop. However, this edge case then needs to be caught on the
    // next accumulation and correct (below).
    if delta > A::default()
      && self.current == A::default()
      && self.current_stop_index >= self.stops.len() - 1
    {
      self.current_stop_index = 0;
    }

    self.current += delta;
    if self.current >= self.target {
      self.current -= self.target;
      self.current_stop_index += 1;

      result.completed_cycle = self.current_stop_index >= max(self.stops.len() - 1, 1);
      result.remainder = self.current;

      if result.completed_cycle {
        match self.cycle {
          AnimationCycle::Forever => {
            if self.current > A::default() {
              self.current_stop_index = 0;
            }
          }
          AnimationCycle::Once => {
            self.cycle_count += 1;
            result.completed_cycle = true;
          }
          AnimationCycle::Times(n) => {
            self.cycle_count += 1;
            if self.cycle_count < n && self.current > A::default() {
              self.current_stop_index = 0;
            }
          }
        }
      }
    }

    result
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count > 0,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn reset(&mut self) {
    self.current = A::default();
    self.cycle_count = 0;
    self.current_stop_index = 0;
  }

  fn force(&mut self, current: A) {
    self.current = current;
  }
}

impl<A, T> Animation<A, T> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync + 'static,
  A: Tweenable
    + Copy
    + Default
    + AddAssign
    + SubAssign
    + PartialEq
    + PartialOrd
    + Send
    + Sync
    + Debug
    + 'static,
{
  fn sample(&self) -> T {
    let phase = (self.current.to_f32() / self.target.to_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    let from = &self.stops[self.current_stop_index];
    let to = &self.stops[self.next_index()];
    from.interpolate(to, curve_value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_once_tween() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], AnimationCycle::Once);
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 5.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 10.0);
    assert!(tween.is_complete());
  }

  #[test]
  fn test_multi_stop_tween() {
    let mut tween = Tween::new(
      1.5,
      Curve::Linear,
      vec![0.0, 10.0, 20.0, 30.0],
      AnimationCycle::Once,
    );
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 10.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 20.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 30.0);
    assert!(tween.is_complete());
  }

  #[test]
  fn test_times_tween() {
    let mut tween = Tween::new(
      1.0,
      Curve::Linear,
      vec![0.0, 10.0],
      AnimationCycle::Times(3),
    );
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 5.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 10.0);
    assert!(!tween.is_complete());
  }

  #[test]
  fn test_forever_tween() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], AnimationCycle::Forever);
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 5.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 10.0);
    assert!(!tween.is_complete());

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 5.0);
  }

  #[test]
  fn test_forever_tween_exact_boundary() {
    let mut tween = Tween::new(
      1.0,
      Curve::Linear,
      vec![0.0, 10.0, 20.0],
      AnimationCycle::Forever,
    );

    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 10.0);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 20.0);

    let result = tween.accumulate(0.25);
    assert_eq!(result.completed_cycle, false);
    assert_eq!(tween.sample(), 5.0);
  }

  #[test]
  fn test_times_overshoot() {
    let mut tween = Tween::new(
      1.0,
      Curve::Linear,
      vec![0.0, 10.0],
      AnimationCycle::Times(3),
    );
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(1.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 5.0);
    assert!(!tween.is_complete());
  }
}
