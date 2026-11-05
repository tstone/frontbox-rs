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

impl<T: Lerp, const N: usize> Lerp for [T; N] {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    std::array::from_fn(|i| self[i].interpolate(&other[i], t))
  }
}

/// Elements past the end of `other` (when it is shorter) keep their value, and extra elements in `other` are ignored,
/// so the result is always the length of `self`
impl<T: Lerp + Clone> Lerp for Vec<T> {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self
      .iter()
      .enumerate()
      .map(|(i, a)| match other.get(i) {
        Some(b) => a.interpolate(b, t),
        None => a.clone(),
      })
      .collect()
  }
}

macro_rules! lerp_tuple {
  ($(($($name:ident $i:tt),+)),*) => {
    $(
      impl<$($name: Lerp),+> Lerp for ($($name,)+) {
        fn interpolate(&self, other: &Self, t: f32) -> Self {
          ($(self.$i.interpolate(&other.$i, t),)+)
        }
      }
    )*
  };
}

lerp_tuple!(
  (A 0),
  (A 0, B 1),
  (A 0, B 1, C 2),
  (A 0, B 1, C 2, D 3),
  (A 0, B 1, C 2, D 3, E 4),
  (A 0, B 1, C 2, D 3, E 4, F 5)
);

#[cfg(feature = "fast-protocol")]
impl Lerp for fast_protocol::Rgb {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let [r, g, b] = [self.r, self.g, self.b].interpolate(&[other.r, other.g, other.b], t);
    fast_protocol::Rgb::new(r, g, b)
  }
}

#[cfg(feature = "image")]
impl Lerp for image::Rgba<u8> {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    image::Rgba(self.0.interpolate(&other.0, t))
  }
}

#[cfg(feature = "image")]
impl Lerp for image::RgbaImage {
  /// Pixels missing from `other` (when it is smaller) interpolate toward transparent
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    let (width, height) = self.dimensions();

    let data = if other.dimensions() == (width, height) {
      // same size, so the raw channel buffers line up one to one
      self
        .as_raw()
        .iter()
        .zip(other.as_raw())
        .map(|(a, b)| a.interpolate(b, t))
        .collect()
    } else {
      let transparent = image::Rgba([0, 0, 0, 0]);
      self
        .enumerate_pixels()
        .flat_map(|(x, y, pixel)| pixel.interpolate(other.get_pixel_checked(x, y).unwrap_or(&transparent), t).0)
        .collect()
    };

    image::RgbaImage::from_raw(width, height, data).expect("interpolated buffer matches the image size")
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

  #[test]
  fn test_floats() {
    assert_eq!(0.0f32.interpolate(&10.0, 0.25), 2.5);
    assert_eq!(10.0f64.interpolate(&0.0, 0.25), 7.5);
    // floats don't clamp, so overshooting curves pass through
    assert_eq!(0.0f32.interpolate(&10.0, 1.5), 15.0);
  }

  #[test]
  fn test_chars() {
    assert_eq!('a'.interpolate(&'e', 0.5), 'c');
    assert_eq!('a'.interpolate(&'e', 1.0), 'e');
  }

  #[test]
  fn test_chars_skip_invalid_code_points() {
    // halfway between these lands in the surrogate range, which isn't a valid char
    assert_eq!('\u{D000}'.interpolate(&'\u{E000}', 0.5), '\u{D000}');
  }

  #[cfg(feature = "image")]
  #[test]
  fn test_pixels() {
    let from = image::Rgba([0, 255, 100, 255]);
    let to = image::Rgba([100, 55, 100, 0]);
    assert_eq!(from.interpolate(&to, 0.5), image::Rgba([50, 155, 100, 127]));
  }

  #[cfg(feature = "fast-protocol")]
  #[test]
  fn test_rgb_in_either_direction() {
    let from = fast_protocol::Rgb::new(200, 0, 100);
    let to = fast_protocol::Rgb::new(0, 200, 100);
    assert_eq!(from.interpolate(&to, 0.5), fast_protocol::Rgb::new(100, 100, 100));
    assert_eq!(to.interpolate(&from, 0.25), fast_protocol::Rgb::new(50, 150, 100));
  }

  #[test]
  fn test_arrays() {
    assert_eq!([0u8, 100, 200].interpolate(&[100, 0, 200], 0.5), [50, 50, 200]);
    assert_eq!([0.0f32; 0].interpolate(&[], 0.5), [0.0f32; 0]);
  }

  #[test]
  fn test_vecs() {
    assert_eq!(vec![0.0f32, 10.0].interpolate(&vec![10.0, 0.0], 0.5), vec![5.0, 5.0]);
  }

  #[test]
  fn test_vecs_keep_the_length_of_self() {
    // elements other doesn't have hold their value
    assert_eq!(vec![0.0f32, 10.0].interpolate(&vec![10.0], 0.5), vec![5.0, 10.0]);
    // extra elements in other are ignored
    assert_eq!(vec![0.0f32].interpolate(&vec![10.0, 20.0], 0.5), vec![5.0]);
  }

  #[test]
  fn test_tuples() {
    assert_eq!((0.0f32,).interpolate(&(10.0,), 0.5), (5.0,));
    assert_eq!((0u8, 0.0f64, 'a').interpolate(&(100, 1.0, 'e'), 0.5), (50, 0.5, 'c'));
    // nested
    assert_eq!(((0u8, 0u8), [0u8; 2]).interpolate(&((10, 20), [30, 40]), 0.5), ((5, 10), [15, 20]));
  }

  #[cfg(feature = "image")]
  #[test]
  fn test_images_of_the_same_size() {
    let from = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0]));
    let to = image::RgbaImage::from_pixel(2, 2, image::Rgba([100, 200, 50, 255]));

    let mid = from.interpolate(&to, 0.5);
    assert_eq!(mid.dimensions(), (2, 2));
    assert!(mid.pixels().all(|p| *p == image::Rgba([50, 100, 25, 128])));
  }

  #[cfg(feature = "image")]
  #[test]
  fn test_images_of_different_sizes_fade_missing_pixels() {
    let from = image::RgbaImage::from_pixel(2, 1, image::Rgba([100, 100, 100, 255]));
    let to = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 200, 200, 255]));

    let mid = from.interpolate(&to, 0.5);
    assert_eq!(mid.dimensions(), (2, 1));
    assert_eq!(*mid.get_pixel(0, 0), image::Rgba([150, 150, 150, 255]));
    assert_eq!(*mid.get_pixel(1, 0), image::Rgba([50, 50, 50, 127]));
  }
}
