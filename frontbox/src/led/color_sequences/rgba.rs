use crate::prelude::*;

impl ColorSequence for Rgba<u8> {
  fn base_render(&self, count: usize) -> Vec<Rgba<u8>> {
    vec![*self; count]
  }

  fn reversed(&self) -> bool {
    false
  }

  fn rotation(&self) -> f32 {
    0.0
  }
}
