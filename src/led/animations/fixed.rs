use std::time::Duration;

use crate::led::animation::{Animation, AnimationCycle};

pub struct Static<T> {
  value: T,
  duration: Duration,
  elapsed: Duration,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<T> Static<T> {
  pub fn new(value: T, duration: Duration, cycle: AnimationCycle) -> Self {
    Self {
      value,
      duration,
      cycle,
      elapsed: Duration::ZERO,
      cycle_count: 0,
    }
  }
}

impl<T> Animation<T> for Static<T>
where
  T: Clone,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    self.elapsed += delta_time;
    if self.elapsed >= self.duration {
      let remainder = self.elapsed - self.duration;
      self.elapsed = self.duration;
      self.cycle_count += 1;

      if self.is_complete() {
        return remainder;
      }
    }
    Duration::ZERO
  }

  fn sample(&self) -> T {
    self.value.clone()
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Times(n) => self.cycle_count >= n && self.elapsed >= self.duration,
      AnimationCycle::Forever => false,
    }
  }
}
