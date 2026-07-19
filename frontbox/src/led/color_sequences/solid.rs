use image::Rgba;

use crate::prelude::ColorSequence;

#[derive(Clone, Debug)]
pub struct Solid {
  pub color: Rgba<u8>,
  pub progress: f32,
}

impl Solid {
  pub fn new(color: Rgba<u8>) -> Self {
    Self {
      color,
      progress: 1.0,
    }
  }
}

impl ColorSequence for Solid {
  fn base_render(&self, count: usize) -> Vec<Rgba<u8>> {
    vec![self.color; count]
  }

  fn reversed(&self) -> bool {
    false
  }

  fn rotation(&self) -> f32 {
    0.0
  }

  fn progress(&self) -> f32 {
    self.progress
  }
}

impl From<Rgba<u8>> for Solid {
  fn from(value: Rgba<u8>) -> Self {
    Solid::new(value)
  }
}
