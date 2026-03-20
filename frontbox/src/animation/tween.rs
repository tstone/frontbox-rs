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

/// Linear interpolation between two values of type T
pub trait Lerp {
  fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for Color {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.mix(other, t)
  }
}

impl Lerp for u8 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as u8
  }
}

impl Lerp for u16 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as u16
  }
}

impl Lerp for u32 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as u32
  }
}

impl Lerp for u64 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f64;
    let to = *other as f64;
    (from + (to - from) * t as f64).round() as u64
  }
}

impl Lerp for usize {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f64;
    let to = *other as f64;
    (from + (to - from) * t as f64).round() as usize
  }
}

impl Lerp for i8 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as i8
  }
}

impl Lerp for i16 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as i16
  }
}

impl Lerp for i32 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f32;
    let to = *other as f32;
    (from + (to - from) * t).round() as i32
  }
}

impl Lerp for i64 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f64;
    let to = *other as f64;
    (from + (to - from) * t as f64).round() as i64
  }
}

impl Lerp for isize {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as f64;
    let to = *other as f64;
    (from + (to - from) * t as f64).round() as isize
  }
}

impl Lerp for f32 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self + (other - self) * t
  }
}

impl Lerp for f64 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self + (other - self) * t as f64
  }
}

impl Lerp for char {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let from = *self as u32;
    let to = *other as u32;
    let interpolated = (from as f64 + (to as f64 - from as f64) * t as f64).round() as u32;
    std::char::from_u32(interpolated).unwrap_or(*self)
  }
}
