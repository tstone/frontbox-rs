use frontbox::prelude::*;

#[derive(Debug, Clone)]
pub enum Fill2d {
  Transparent,
  Solid(Rgba<u8>),
  Gradient(Vec<GradientStop>, f32),
}

#[derive(Debug, Clone, Copy)]
pub struct Border {
  pub color: Rgba<u8>,
  pub width: u8,
}

impl Border {
  pub fn new(width: u8, color: Rgba<u8>) -> Self {
    Self { color, width }
  }
}
