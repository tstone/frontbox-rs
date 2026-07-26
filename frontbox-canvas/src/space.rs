use std::fmt::Debug;

#[derive(Debug, Default, Clone, Copy)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}

impl Position {
  pub fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub fn from_u32(x: u32, y: u32) -> Self {
    Self {
      x: x as i32,
      y: y as i32,
    }
  }

  pub fn zero() -> Self {
    Self::default()
  }
}

#[derive(Debug, Default, Clone)]
pub struct Size<T: Debug + Clone> {
  pub width: T,
  pub height: T,
}

impl<T> Size<T>
where
  T: Debug + Clone,
{
  pub fn new(width: T, height: T) -> Self {
    Self { width, height }
  }
}

impl<T> Copy for Size<T> where T: Debug + Clone + Copy {}
