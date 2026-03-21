use std::time::Duration;

pub trait ToF32 {
  fn to_f32(&self) -> f32;
}

impl ToF32 for u8 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for i8 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for i16 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for u16 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for i32 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for u32 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for f32 {
  fn to_f32(&self) -> f32 {
    *self
  }
}

impl ToF32 for f64 {
  fn to_f32(&self) -> f32 {
    *self as f32
  }
}

impl ToF32 for Duration {
  fn to_f32(&self) -> f32 {
    self.as_secs_f32()
  }
}
