use image::Rgba;

use crate::prelude::*;

pub struct Tile {
  pub seq: Vec<Rgba<u8>>,
}

impl ColorSequence for Tile {
  fn render(&self, count: usize) -> Vec<image::Rgba<u8>> {
    let mut colors = vec![Rgba::default(); count];

    if self.seq.len() > 0 {
      let mut pointer: usize = 0;

      for index in 0..count {
        colors[index] = self.seq[pointer];

        pointer += 1;
        if pointer >= self.seq.len() {
          pointer = 0;
        }
      }
    }

    colors
  }
}
