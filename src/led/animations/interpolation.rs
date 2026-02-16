use std::time::Duration;

use palette::Mix;
use palette::Srgb;

use crate::led::animation::{Animation, AnimationCycle};
use crate::led::curve::Curve;

/// Animation implementation that Lerps between two values of type T over a specified duration using a given curve
pub struct Interpolation<T> {
  duration: Duration,
  elapsed: Duration,
  curve: Curve,
  from: T,
  to: T,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<T> Animation<T> for Interpolation<T>
where
  T: Lerp,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    self.elapsed += delta_time;
    if self.elapsed >= self.duration {
      let remainder = self.elapsed - self.duration;
      self.elapsed = self.duration;
      remainder
    } else {
      Duration::ZERO
    }
  }

  fn sample(&self) -> T {
    let phase = (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    self.from.interpolate(&self.to, curve_value)
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Times(n) => self.cycle_count >= n && self.elapsed >= self.duration,
      AnimationCycle::Forever => false,
    }
  }
}

/// Linear interpolation between two values of type T
pub trait Lerp {
  fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for Srgb {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.mix(*other, t)
  }
}
