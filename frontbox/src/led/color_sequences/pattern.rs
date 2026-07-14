use image::Rgba;

use crate::prelude::*;

pub struct Pattern {
  pub seq: Vec<Rgba<u8>>,
  pub index: u16,
  pub other: Option<Box<dyn ColorSequence>>,
}

impl ColorSequence for Pattern {
  fn render(&self, count: usize) -> Vec<image::Rgba<u8>> {
    let mut colors = match &self.other {
      Some(other) => other.render(count),
      None => (0..count).map(|_| Rgba::default()).collect(),
    };

    for (offset, pixel) in self.seq.iter().enumerate() {
      let index = self.index as usize + offset;

      if index < count {
        if index >= colors.len() {
          colors.push(*pixel);
        } else {
          colors[index] = *pixel;
        }
      }
    }

    colors
  }
}
