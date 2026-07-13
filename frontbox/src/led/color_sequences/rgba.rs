use crate::prelude::*;

impl ColorSequence for Rgba<u8> {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    vec![*self; count]
  }
}

impl ColorSequence for Vec<Rgba<u8>> {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    let mut colors = vec![Rgba::default(); count];
    for (index, pixel) in self.iter().enumerate() {
      if index < count {
        colors[index] = *pixel;
      }
    }
    colors
  }
}
