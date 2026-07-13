use crate::prelude::*;

pub struct Reverse {
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for Reverse {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    self.other.render(count).into_iter().rev().collect()
  }
}
