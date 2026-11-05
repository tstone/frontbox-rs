/// Linear interpolation between two values of type T
pub trait Lerp {
  fn interpolate(&self, other: &Self, t: f32) -> Self;
}

/// Integers move by an offset from `self`, so `t = 0` returns `self` exactly, and results past either end (e.g. from
/// `Curve::BackOut`) clamp to the type's range
macro_rules! lerp_integer {
  ($($t:ty),*) => {
    $(
      impl Lerp for $t {
        fn interpolate(&self, other: &Self, t: f32) -> Self {
          let from = *self as i128;
          let offset = ((*other as i128 - from) as f64 * t as f64).round() as i128;
          (from + offset).clamp(<$t>::MIN as i128, <$t>::MAX as i128) as $t
        }
      }
    )*
  };
}

lerp_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

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

#[cfg(feature = "fast-protocol")]
impl Lerp for fast_protocol::Color {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.mix(other, t)
  }
}

#[cfg(feature = "image")]
impl Lerp for image::Rgba<u8> {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    image::Rgba([
      self[0].interpolate(&other[0], t),
      self[1].interpolate(&other[1], t),
      self[2].interpolate(&other[2], t),
      self[3].interpolate(&other[3], t),
    ])
  }
}

#[cfg(feature = "image")]
impl Lerp for Vec<image::Rgba<u8>> {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self
      .iter()
      .enumerate()
      .map(|(i, pixel)| pixel.interpolate(other.get(i).unwrap_or(&image::Rgba([0, 0, 0, 0])), t))
      .collect()
  }
}

#[cfg(feature = "image")]
impl Lerp for image::RgbaImage {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let mut buffer = image::RgbaImage::new(self.width(), self.height());

    for x in 0..self.width() {
      for y in 0..self.height() {
        let p = self.get_pixel(x, y);
        let other = match other.get_pixel_checked(x, y) {
          Some(p) => p,
          None => &image::Rgba([0, 0, 0, 0]),
        };
        buffer.put_pixel(x, y, p.interpolate(other, t));
      }
    }

    buffer
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_large_integers_keep_precision() {
    assert_eq!(100_000_000u32.interpolate(&100_000_010, 0.5), 100_000_005);
    assert_eq!(1_000_000_000_000i64.interpolate(&1_000_000_000_010, 0.5), 1_000_000_000_005);
    assert_eq!(u64::MAX.interpolate(&(u64::MAX - 10), 0.5), u64::MAX - 5);
  }

  #[test]
  fn test_integers_clamp_past_either_end() {
    assert_eq!(0u8.interpolate(&255, 1.2), 255);
    assert_eq!(0u8.interpolate(&255, -0.2), 0);
    assert_eq!(10i8.interpolate(&20, 0.5), 15);
  }
}
