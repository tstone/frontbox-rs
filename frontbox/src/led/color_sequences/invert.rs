use crate::prelude::*;

pub struct Invert {
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for Invert {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    self
      .other
      .render(count)
      .into_iter()
      .map(|Rgba([r, g, b, a])| Rgba([255 - r, 255 - g, 255 - b, a]))
      .collect()
  }
}
