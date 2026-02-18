use std::time::Duration;

use crate::led::animation::{Animation, AnimationCycle};
use crate::led::curve::Curve;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified duration using a given curve
#[derive(Clone)]
pub struct InterpolationAnimation<T> {
  duration: Duration,
  elapsed: Duration,
  curve: Curve,
  from: T,
  to: T,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<T> InterpolationAnimation<T> {
  pub fn new(duration: Duration, curve: Curve, from: T, to: T, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self {
      duration,
      elapsed: Duration::ZERO,
      curve,
      from,
      to,
      cycle,
      cycle_count: 0,
    })
  }

  pub fn flash(hz: f32, color: T, cycle: AnimationCycle) -> Box<Self>
  where
    T: Default,
  {
    Self::new(
      Duration::from_millis((1000.0 / hz) as u64),
      Curve::ExponentialInOut,
      T::default(),
      color,
      cycle,
    )
  }
}

impl<T> Animation<T> for InterpolationAnimation<T>
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
        return self.elapsed;
      }
    }

    Duration::ZERO
  }

  fn sample(&self) -> T {
    let phase = (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    self.from.interpolate(&self.to, curve_value)
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
  }
}

/// Linear interpolation between two values of type T
pub trait Lerp {
  fn interpolate(&self, other: &Self, t: f32) -> Self;
}
