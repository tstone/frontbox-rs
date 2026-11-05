use std::time::Duration;

/// An amount a [`Tween`](crate::Tween) can be advanced by, usually time
pub trait Tweenable: Sized {
  fn to_f64(&self) -> f64;
}

macro_rules! tweenable_number {
  ($($t:ty),*) => {
    $(
      impl Tweenable for $t {
        fn to_f64(&self) -> f64 {
          *self as f64
        }
      }
    )*
  };
}

tweenable_number!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

impl Tweenable for Duration {
  fn to_f64(&self) -> f64 {
    self.as_secs_f64()
  }
}
