use std::fmt::Debug;
use std::ops::{Add, Sub};

use crate::*;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified quantity using a given curve
#[derive(Clone)]
pub struct Tween<A: Tweenable + Copy + Default + Debug, T: Lerp + Clone + Send + Sync> {
  // the amount which signals this animation is done
  pub target: A,
  pub active: bool,
  current: A,
  pub curve: Curve,
  pub stops: Vec<T>,
  pub cycle: Cycle,
  cycle_count: u32,
  current_stop_index: usize,
}

impl<A, T> Tween<A, T>
where
  T: Lerp + Clone + Send + Sync + 'static,
  A: Tweenable
    + Copy
    + Default
    + Add<Output = A>
    + Sub<Output = A>
    + PartialEq
    + PartialOrd
    + Send
    + Sync
    + Debug
    + 'static,
{
  /// *target* - Accumulated value to get to, e.g. Duration::from_secs(1) = animation lasts for a minute
  pub fn new(target: A, curve: Curve, stops: Vec<T>, cycle: Cycle) -> Self {
    assert!(stops.len() >= 2, "Tween requires at least 2 stops");

    Self {
      target: target.div_usize(stops.len() - 1),
      current: A::default(),
      active: true,
      curve,
      stops,
      cycle,
      cycle_count: 0,
      current_stop_index: 0,
    }
  }

  pub fn once(target: A, curve: Curve, stops: Vec<T>) -> Self {
    Self::new(target, curve, stops, Cycle::Once)
  }

  pub fn forever(target: A, curve: Curve, stops: Vec<T>) -> Self {
    Self::new(target, curve, stops, Cycle::Forever)
  }

  pub fn linear(target: A, stops: Vec<T>, cycle: Cycle) -> Self {
    Self::new(target, Curve::Linear, stops, cycle)
  }

  /// Tween starts in a playing state by default. Use this to chain it to prevent that.
  pub fn stopped(mut self) -> Self {
    self.stop();
    self
  }

  /// Plays the stops forward, then back again, as one cycle
  pub fn ping_pong(target: A, curve: Curve, stops: Vec<T>, cycle: Cycle) -> Sequence<A, T> {
    let forward = Tween::new(target, curve, stops, Cycle::Once);
    let mut backward = forward.clone();
    backward.reverse();

    Sequence::new(vec![Box::new(forward), Box::new(backward)], cycle)
  }

  pub fn boxed(target: A, curve: Curve, stops: Vec<T>, cycle: Cycle) -> Box<Self> {
    Box::new(Self::new(target, curve, stops, cycle))
  }

  fn last_stop(&self) -> usize {
    self.stops.len() - 1
  }

  fn next_index(&self) -> usize {
    let idx = self.current_stop_index + 1;
    if idx == self.stops.len() {
      return 0;
    }
    idx
  }

  /// Turns the tween into the original played backwards: the stops run in reverse order, and each segment's curve
  /// runs backwards in time (an ease-in becomes an ease-out). Progress and completed cycles are reset, so it plays
  /// from the start of the reversed path.
  pub fn reverse(&mut self) {
    self.stops.reverse();
    self.curve = self.curve.clone().reflect();
    self.reset();
  }

  pub fn remaining(&self) -> A {
    self.target - self.current
  }
}

impl<A, T> Accumulator<A> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: Tweenable
    + Copy
    + Default
    + Add<Output = A>
    + Sub<Output = A>
    + PartialOrd
    + PartialEq
    + Send
    + Sync
    + Debug,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    if !self.active || self.is_complete() {
      return AccumulationResult::default();
    }

    let mut result = AccumulationResult::default();
    let last_stop = self.stops.len() - 1;

    // A zero-length tween completes as soon as it's advanced
    if self.target <= A::default() {
      self.current_stop_index = last_stop;
      self.cycle_count = match self.cycle {
        Cycle::Once => 1,
        Cycle::Times(n) => n,
        Cycle::Forever => self.cycle_count,
      };
      result.completed_cycle = true;
      result.remainder = delta;
      return result;
    }

    // When a cycle ends exactly on the target, the tween parks on the last stop so `sample()` returns it rather than
    // jumping back to the first. The next advance starts the new cycle.
    if delta > A::default() && self.current == A::default() && self.current_stop_index >= last_stop {
      self.current_stop_index = 0;
    }

    self.current = self.current + delta;

    // A single delta may cover several segments, or several cycles
    while self.current >= self.target {
      self.current = self.current - self.target;
      self.current_stop_index += 1;

      if self.current_stop_index < last_stop {
        continue;
      }

      result.completed_cycle = true;
      if self.cycle != Cycle::Forever {
        self.cycle_count = self.cycle_count.saturating_add(1);
      }

      if self.is_complete() {
        result.remainder = self.current;
        self.current = A::default();
        break;
      }

      if self.current > A::default() {
        self.current_stop_index = 0;
      }
    }

    result
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      Cycle::Once => self.cycle_count > 0,
      Cycle::Times(n) => self.cycle_count >= n,
      Cycle::Forever => false,
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
    + Add<Output = A>
    + Sub<Output = A>
    + PartialEq
    + PartialOrd
    + Send
    + Sync
    + Debug
    + 'static,
{
  fn sample(&self) -> T {
    // Parked at the end of the last segment, either between cycles or complete
    if self.current_stop_index >= self.last_stop() {
      let last = self.last_stop();
      return self.stops[last - 1].interpolate(&self.stops[last], self.curve.sample(1.0));
    }

    let phase = (self.current.to_f32() / self.target.to_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    let from = &self.stops[self.current_stop_index];
    let to = &self.stops[self.next_index()];
    from.interpolate(to, curve_value)
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
  fn test_once_tween() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once);
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
    let mut tween = Tween::new(1.5, Curve::Linear, vec![0.0, 10.0, 20.0, 30.0], Cycle::Once);
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
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Times(3));
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
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Forever);
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
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0, 20.0], Cycle::Forever);

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
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Times(3));
    assert_eq!(tween.sample(), 0.0);

    let result = tween.accumulate(1.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 5.0);
    assert!(!tween.is_complete());
  }

  #[test]
  fn test_delta_spanning_several_stops() {
    let mut tween = Tween::new(3.0, Curve::Linear, vec![0.0, 10.0, 20.0, 30.0], Cycle::Once);

    let result = tween.accumulate(3.0);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 30.0);
    assert!(tween.is_complete());
  }

  #[test]
  fn test_delta_spanning_several_cycles() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Forever);

    let result = tween.accumulate(2.25);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(tween.sample(), 2.5);
  }

  #[test]
  fn test_times_completes_within_one_delta() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Times(2));

    let result = tween.accumulate(2.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(result.remainder, 0.5);
    assert_eq!(tween.sample(), 10.0);
    assert!(tween.is_complete());
  }

  #[test]
  fn test_once_overshoot_holds_last_stop() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once);

    let result = tween.accumulate(1.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(result.remainder, 0.5);
    assert_eq!(tween.sample(), 10.0);

    tween.accumulate(1.0);
    assert_eq!(tween.sample(), 10.0);
  }

  #[test]
  fn test_force_beyond_a_segment_catches_up() {
    let mut tween = Tween::new(2.0, Curve::Linear, vec![0.0, 10.0, 20.0], Cycle::Once);

    tween.force(1.5);
    tween.accumulate(0.0);
    assert_eq!(tween.sample(), 15.0);
  }

  #[test]
  fn test_parked_sample_follows_curve() {
    // Sinusoid returns to its start at the end of each cycle
    let mut tween = Tween::new(1.0, Curve::Sinusoid, vec![0.0f32, 10.0], Cycle::Forever);

    tween.accumulate(1.0);
    assert!(tween.sample().abs() < 0.001);
  }

  #[test]
  fn test_zero_length_tween_completes() {
    let mut tween = Tween::new(0.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once);

    let result = tween.accumulate(0.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(result.remainder, 0.5);
    assert_eq!(tween.sample(), 10.0);
    assert!(tween.is_complete());
  }

  #[test]
  fn test_reverse_plays_backwards() {
    let curve = Curve::EaseIn;
    let forward = Tween::new(3.0, curve.clone(), vec![0.0f32, 10.0, 20.0, 30.0], Cycle::Once);
    let mut backward = forward.clone();
    backward.reverse();

    for i in 0..=30 {
      let t = i as f32 / 10.0;
      let mut f = forward.clone();
      let mut b = backward.clone();
      f.accumulate(t);
      b.accumulate(3.0 - t);
      assert!((f.sample() - b.sample()).abs() < 0.001, "at {t}: forward {} backward {}", f.sample(), b.sample());
    }
  }

  #[test]
  fn test_reverse_resets_progress() {
    let mut tween = Tween::new(1.0, Curve::Linear, vec![0.0f32, 10.0], Cycle::Once);
    tween.accumulate(1.0);
    assert!(tween.is_complete());

    tween.reverse();
    assert!(!tween.is_complete());
    assert_eq!(tween.sample(), 10.0);

    tween.accumulate(1.0);
    assert_eq!(tween.sample(), 0.0);
  }

  #[test]
  fn test_ping_pong_returns_the_way_it_came() {
    let mut seq = Tween::ping_pong(1.0, Curve::EaseIn, vec![0.0f32, 10.0], Cycle::Once);

    seq.accumulate(0.5);
    let out = seq.sample();
    assert_eq!(out, 2.5);

    seq.accumulate(1.0);
    assert_eq!(seq.sample(), out);

    seq.accumulate(0.5);
    assert!(seq.is_complete());
    assert_eq!(seq.sample(), 0.0);
  }
}
