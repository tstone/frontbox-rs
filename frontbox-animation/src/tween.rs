use crate::*;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified quantity using a given curve
#[derive(Clone)]
pub struct Tween<A, T> {
  /// The accumulated amount for one cycle through all of the stops
  pub target: A,
  pub active: bool,
  pub curve: Curve,
  pub stops: Vec<T>,
  pub cycle: Cycle,
  /// Progress through the current cycle, from zero up to `target`
  current: A,
  cycle_count: u32,
  /// Set when a cycle ends exactly on `target`, or the tween is complete, so `sample()` holds the end of the last stop
  /// rather than jumping back to the first. The next advance starts the new cycle.
  at_end: bool,
}

impl<A, T> Tween<A, T>
where
  T: Lerp + Clone + Send + Sync + 'static,
  A: Tweenable + 'static,
{
  /// *target* - The accumulated amount for one cycle through all of the stops, e.g. `Duration::from_secs(1)` moves
  /// through every stop in one second
  pub fn new(target: A, curve: Curve, stops: Vec<T>, cycle: Cycle) -> Self {
    assert!(stops.len() >= 2, "Tween requires at least 2 stops");

    Self {
      target,
      current: A::default(),
      active: true,
      curve,
      stops,
      cycle,
      cycle_count: 0,
      at_end: false,
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

  /// Turns the tween into the original played backwards: the stops run in reverse order, and each segment's curve
  /// runs backwards in time (an ease-in becomes an ease-out). Progress and completed cycles are reset, so it plays
  /// from the start of the reversed path.
  pub fn reverse(&mut self) {
    self.stops.reverse();
    self.curve = self.curve.clone().reflect();
    self.reset();
  }

  /// The amount left in the current cycle, or zero once the tween is complete
  pub fn remaining(&self) -> A {
    if self.is_complete() {
      return A::default();
    }
    self.target - self.current
  }
}

impl<A, T> Accumulator<A> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: Tweenable,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    if !self.active || self.is_complete() {
      return AccumulationResult::default();
    }

    let mut result = AccumulationResult::default();

    // A zero-length tween completes as soon as it's advanced
    if self.target <= A::default() {
      self.at_end = true;
      self.cycle_count = match self.cycle {
        Cycle::Once => 1,
        Cycle::Times(n) => n,
        Cycle::Forever => self.cycle_count,
      };
      result.completed_cycle = true;
      result.remainder = delta;
      return result;
    }

    if delta > A::default() {
      self.at_end = false;
    }

    self.current = self.current + delta;

    // A single delta may cover several cycles
    while self.current >= self.target {
      self.current = self.current - self.target;
      result.completed_cycle = true;
      if self.cycle != Cycle::Forever {
        self.cycle_count = self.cycle_count.saturating_add(1);
      }

      if self.is_complete() {
        result.remainder = self.current;
        self.current = A::default();
        self.at_end = true;
        break;
      }

      if self.current == A::default() {
        self.at_end = true;
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
    self.at_end = false;
  }

  fn force(&mut self, current: A) {
    self.current = current;
    self.at_end = false;
  }
}

impl<A, T> Animation<A, T> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync + 'static,
  A: Tweenable + 'static,
{
  fn sample(&self) -> T {
    let segments = self.stops.len() - 1;

    if self.at_end {
      return self.stops[segments - 1].interpolate(&self.stops[segments], self.curve.sample(1.0));
    }

    // Position across all of the stops, e.g. 1.5 is halfway between the second and third
    let target = self.target.to_f64();
    let position = if target > 0.0 {
      self.current.to_f64() * segments as f64 / target
    } else {
      0.0
    };
    let segment = (position as usize).min(segments - 1);
    let phase = (position - segment as f64).min(1.0) as f32;

    self.stops[segment].interpolate(&self.stops[segment + 1], self.curve.sample(phase))
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

  #[test]
  fn test_target_is_the_whole_cycle() {
    let mut tween = Tween::new(3.0, Curve::Linear, vec![0.0f32, 10.0, 20.0, 30.0], Cycle::Once);
    assert_eq!(tween.target, 3.0);
    assert_eq!(tween.remaining(), 3.0);

    tween.accumulate(1.5);
    assert_eq!(tween.remaining(), 1.5);

    tween.accumulate(1.5);
    assert_eq!(tween.remaining(), 0.0);
  }

  #[test]
  fn test_integer_target_is_not_truncated() {
    let mut tween = Tween::new(10u32, Curve::Linear, vec![0.0f32, 1.0, 2.0, 3.0], Cycle::Once);

    tween.accumulate(9);
    assert!(!tween.is_complete());
    assert!((tween.sample() - 2.7).abs() < 0.001);

    tween.accumulate(1);
    assert!(tween.is_complete());
    assert_eq!(tween.sample(), 3.0);
  }
}
