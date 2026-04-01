use std::time::Duration;

pub trait Tweenable: Sized {
  fn to_f32(&self) -> f32;
  fn div_usize(self, rhs: usize) -> Self;
}

impl Tweenable for u8 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as u8
  }
}

impl Tweenable for i8 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as i8
  }
}

impl Tweenable for i16 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as i16
  }
}

impl Tweenable for u16 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as u16
  }
}

impl Tweenable for i32 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as i32
  }
}

impl Tweenable for u32 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as u32
  }
}

impl Tweenable for f32 {
  fn to_f32(&self) -> f32 {
    *self
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as f32
  }
}

impl Tweenable for f64 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as f64
  }
}

impl Tweenable for Duration {
  fn to_f32(&self) -> f32 {
    self.as_secs_f32()
  }
  fn div_usize(self, rhs: usize) -> Self {
    self / rhs as u32
  }
}
