/// Captures a description of a position or length, either absolute (fixed) or relative (percent)
#[derive(Debug, Clone, Copy)]
pub enum Extent {
  /// Values 0.0..=1.0 where 0.0 represents 0% and 1.0 represents 100%
  Relative(f32),
  Absolute(usize),
}

impl Extent {
  pub fn zero() -> Self {
    Self::Absolute(0)
  }

  pub fn full() -> Self {
    Self::Relative(100.0)
  }

  pub fn to_absolute(&self, length: usize) -> usize {
    match self {
      Self::Absolute(i) => *i,
      Self::Relative(p) => (length as f32 * p) as usize,
    }
  }

  pub fn to_relative(&self, length: usize) -> f32 {
    match self {
      Self::Absolute(i) => *i as f32 / length as f32,
      Self::Relative(p) => *p,
    }
  }

  pub fn relative_mut(&mut self) -> Option<&mut f32> {
    match self {
      Self::Relative(v) => Some(v),
      _ => None,
    }
  }

  pub fn absolute_mut(&mut self) -> Option<&mut usize> {
    match self {
      Self::Absolute(v) => Some(v),
      _ => None,
    }
  }
}

impl From<u8> for Extent {
  fn from(value: u8) -> Self {
    Self::Absolute(value as usize)
  }
}

impl From<u16> for Extent {
  fn from(value: u16) -> Self {
    Self::Absolute(value as usize)
  }
}

impl From<u32> for Extent {
  fn from(value: u32) -> Self {
    Self::Absolute(value as usize)
  }
}

impl From<u64> for Extent {
  fn from(value: u64) -> Self {
    Self::Absolute(value as usize)
  }
}

impl From<f32> for Extent {
  fn from(value: f32) -> Self {
    Self::Relative(value)
  }
}

impl From<f64> for Extent {
  fn from(value: f64) -> Self {
    Self::Relative(value as f32)
  }
}
