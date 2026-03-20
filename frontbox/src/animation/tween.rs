use fast_protocol::Color;
use std::time::Duration;

use crate::animation::*;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified duration using a given curve
#[derive(Clone)]
pub struct Tween<T: Lerp + Clone + Send + Sync> {
  pub duration: Duration,
  elapsed: Duration,
  pub curve: Curve,
  pub stops: Vec<T>,
  pub cycle: AnimationCycle,
  cycle_count: u32,
  current_stop_index: usize,
}

impl<T> Tween<T>
where
  T: Lerp + Clone + Send + Sync,
{
  pub fn new(duration: Duration, curve: Curve, stops: Vec<T>, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self {
      duration,
      elapsed: Duration::ZERO,
      curve,
      stops,
      cycle,
      cycle_count: 0,
      current_stop_index: 0,
    })
  }

  fn next_index(&self) -> usize {
    (self.current_stop_index + 1) % self.stops.len()
  }

  pub fn reverse(&mut self) {
    Tween {
      duration: self.duration,
      elapsed: self.elapsed,
      curve: Curve::Reverse(Box::new(self.curve.clone())),
      stops: self.stops.clone().into_iter().rev().collect(),
      cycle: self.cycle.clone(),
      cycle_count: self.cycle_count,
      current_stop_index: self.current_stop_index,
    };
  }
}

impl<T> Animation<T> for Tween<T>
where
  T: Lerp + Clone + Send + Sync,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    self.elapsed += delta_time;
    if self.elapsed >= self.duration {
      if self.cycle != AnimationCycle::Forever && self.cycle_count < u32::MAX {
        self.cycle_count += 1;
      }

      if !self.is_complete() {
        self.elapsed = self.elapsed - self.duration;
        self.current_stop_index = self.next_index();
        return self.elapsed;
      }
    }

    Duration::ZERO
  }

  fn sample(&self) -> T {
    let phase = (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    let from = &self.stops[self.current_stop_index];
    let to = &self.stops[self.next_index()];
    from.interpolate(to, curve_value)
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count > 0,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
    self.cycle_count = 0;
    self.current_stop_index = 0;
  }
}
