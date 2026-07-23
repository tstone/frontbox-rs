use frontbox::prelude::color_sequence::Extent;

#[derive(Debug, Clone, Copy)]
pub enum Horizontal {
  Centered,
  LeftOffset(Extent<i32>),
  RightOffset(Extent<i32>),
}

impl Horizontal {
  pub fn zero() -> Self {
    Self::LeftOffset(Extent::zero())
  }

  pub fn left_mut(&mut self) -> Option<&mut Extent<i32>> {
    match self {
      Self::LeftOffset(l) => Some(l),
      _ => None,
    }
  }

  pub fn right_mut(&mut self) -> Option<&mut Extent<i32>> {
    match self {
      Self::RightOffset(r) => Some(r),
      _ => None,
    }
  }
}

impl Default for Horizontal {
  fn default() -> Self {
    Self::zero()
  }
}

#[derive(Debug, Clone, Copy)]
pub enum Vertical {
  Centered,
  TopOffset(Extent<i32>),
  BottomOffset(Extent<i32>),
}

impl Vertical {
  pub fn zero() -> Self {
    Self::TopOffset(Extent::zero())
  }

  pub fn top_mut(&mut self) -> Option<&mut Extent<i32>> {
    match self {
      Self::TopOffset(t) => Some(t),
      _ => None,
    }
  }

  pub fn bottom_mut(&mut self) -> Option<&mut Extent<i32>> {
    match self {
      Self::BottomOffset(b) => Some(b),
      _ => None,
    }
  }
}

impl Default for Vertical {
  fn default() -> Self {
    Self::zero()
  }
}
