use num_traits::AsPrimitive;

use crate::animation::Lerp;

/// Captures a description of a position or length, either absolute (fixed) or relative (percent).
/// Extents are used by the framework in places where the actual value is later determined at some future computational point.
/// 
/// ```rust
/// Extent::relative(0.5) // half way, 50%
/// Extent::absolute(2) // concretely at 2
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Extent<T: Copy> {
  /// Values 0.0..=1.0 where 0.0 represents 0% and 1.0 represents 100%
  Relative(f32),
  Absolute(T),
}

impl<T> Extent<T>
where
  T: 'static + Default + Copy + AsPrimitive<f32>,
  f32: AsPrimitive<T>,
{
  pub fn zero() -> Self {
    Self::Absolute(T::default())
  }

  pub fn full() -> Self {
    Self::Relative(1.0)
  }

  pub fn fixed(t: T) -> Self {
    Self::Absolute(t)
  }

  pub fn percent(p: f32) -> Self {
    Self::Relative(p)
  }

  pub fn to_absolute(&self, full: T) -> T {
    match self {
      Self::Absolute(i) => *i,
      Self::Relative(p) => (full.as_() * p).as_(),
    }
  }

  pub fn to_relative(&self, full: T) -> f32 {
    match self {
      Self::Absolute(i) => i.as_() / full.as_(),
      Self::Relative(p) => *p,
    }
  }

  pub fn relative_mut(&mut self) -> Option<&mut f32> {
    match self {
      Self::Relative(v) => Some(v),
      _ => None,
    }
  }

  pub fn absolute_mut(&mut self) -> Option<&mut T> {
    match self {
      Self::Absolute(v) => Some(v),
      _ => None,
    }
  }
}

impl<T: Copy> Lerp for Extent<T>
where
  T: Lerp,
{
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Extent::Relative(a), Extent::Relative(b)) => Extent::Relative(a + (b - a) * t),
      (Extent::Absolute(a), Extent::Absolute(b)) => Extent::Absolute(a.interpolate(b, t)),
      _ => {
        if t < 0.5 {
          *self
        } else {
          *other
        }
      }
    }
  }
}

impl<T> Default for Extent<T>
where
  T: Default + Copy,
{
  fn default() -> Self {
    Self::Absolute(T::default())
  }
}

impl<T> From<u8> for Extent<T>
where
  T: Default + Copy + 'static,
  u8: AsPrimitive<T>,
{
  fn from(value: u8) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<u16> for Extent<T>
where
  T: Default + Copy + 'static,
  u16: AsPrimitive<T>,
{
  fn from(value: u16) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<u32> for Extent<T>
where
  T: Default + Copy + 'static,
  u32: AsPrimitive<T>,
{
  fn from(value: u32) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<u64> for Extent<T>
where
  T: Default + Copy + 'static,
  u64: AsPrimitive<T>,
{
  fn from(value: u64) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<usize> for Extent<T>
where
  T: Default + Copy + 'static,
  usize: AsPrimitive<T>,
{
  fn from(value: usize) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<i8> for Extent<T>
where
  T: Default + Copy + 'static,
  i8: AsPrimitive<T>,
{
  fn from(value: i8) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<i16> for Extent<T>
where
  T: Default + Copy + 'static,
  i16: AsPrimitive<T>,
{
  fn from(value: i16) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<i32> for Extent<T>
where
  T: Default + Copy + 'static,
  i32: AsPrimitive<T>,
{
  fn from(value: i32) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<i64> for Extent<T>
where
  T: Default + Copy + 'static,
  i64: AsPrimitive<T>,
{
  fn from(value: i64) -> Self {
    Self::Absolute(value.as_())
  }
}

impl<T> From<f32> for Extent<T>
where
  T: Copy + 'static,
{
  fn from(value: f32) -> Self {
    Self::Relative(value)
  }
}

impl<T> From<f64> for Extent<T>
where
  T: Copy + 'static,
{
  fn from(value: f64) -> Self {
    Self::Relative(value as f32)
  }
}
