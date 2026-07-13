use image::Rgba;

/// A description of a sequence of colors, given a of colors to generate
pub trait ColorSequence {
  fn render(&self, count: usize) -> Vec<Rgba<u8>>;
}

impl ColorSequence for Rgba<u8> {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    vec![*self; count]
  }
}
