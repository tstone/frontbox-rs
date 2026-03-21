use fast_protocol::Color;

#[cfg(feature = "image")]
use image::Rgba;

/// Linear interpolation between two values of type T
pub trait Lerp {
  fn interpolate(&self, other: &Self, t: f32) -> Self;
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

impl Lerp for Color {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.mix(other, t)
  }
}

#[cfg(feature = "image")]
impl Lerp for Rgba<u8> {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    Rgba([
      self[0].interpolate(&other[0], t),
      self[1].interpolate(&other[1], t),
      self[2].interpolate(&other[2], t),
      self[3].interpolate(&other[3], t),
    ])
  }
}
