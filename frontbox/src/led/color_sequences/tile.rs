use image::Rgba;

use crate::prelude::*;

#[derive(Clone)]
pub struct Tile {
  pub seq: Vec<Rgba<u8>>,
  pub rotation: f32,
  pub reversed: bool,
  pub progress: f32,
}

impl Tile {
  pub fn new(pattern: Vec<Rgba<u8>>) -> Self {
    Tile {
      seq: pattern,
      rotation: 0.0,
      reversed: false,
      progress: 1.0,
    }
  }
}

impl ColorSequence for Tile {
  fn base_render(&self, count: usize) -> Vec<image::Rgba<u8>> {
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

  fn reversed(&self) -> bool {
    self.reversed
  }

  fn rotation(&self) -> f32 {
    self.rotation
  }

  fn progress(&self) -> f32 {
    self.progress
  }
}
