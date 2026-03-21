use std::time::Duration;

use crate::time::*;

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

impl<T> Tickable for Tween<T>
where
  T: Lerp + Clone + Send + Sync,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    if self.is_complete() {
      return Duration::ZERO;
    }

    self.elapsed += delta_time;
    if self.elapsed >= self.duration {
      self.elapsed -= self.duration;
      let is_last_stop = self.current_stop_index == self.stops.len() - 2;

      if is_last_stop {
        match self.cycle {
          AnimationCycle::Forever => {
            self.current_stop_index = 0;
          }
          AnimationCycle::Once => {
            self.cycle_count += 1;
            self.elapsed = self.duration; // clamp to end
          }
          AnimationCycle::Times(n) => {
            self.cycle_count += 1;
            if self.cycle_count < n {
              self.current_stop_index = 0;
            } else {
              self.elapsed = self.duration; // clamp to end
            }
          }
        }
      } else {
        self.current_stop_index += 1;
      }

      return self.elapsed;
    }

    Duration::ZERO
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count > 0,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn completed_this_tick(&self) -> bool {
    self.is_complete() && self.elapsed <= self.duration
  }

  fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
    self.cycle_count = 0;
    self.current_stop_index = 0;
  }
}

impl<T> Animation<T> for Tween<T>
where
  T: Lerp + Clone + Send + Sync,
{
  fn sample(&self) -> T {
    let phase = (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    let from = &self.stops[self.current_stop_index];
    let to = &self.stops[self.next_index()];
    from.interpolate(to, curve_value)
  }
}
