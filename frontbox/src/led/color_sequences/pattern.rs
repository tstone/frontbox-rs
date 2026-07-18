use image::Rgba;

use crate::prelude::*;

#[derive(Clone)]
pub struct Pattern {
  pub seq: Vec<Rgba<u8>>,
  pub insert_index: u16,
  pub rotation: f32,
  pub reversed: bool,
}

impl Pattern {
  pub fn new(pattern: Vec<Rgba<u8>>) -> Self {
    Self::at(0, pattern)
  }

  pub fn at(insert_index: u16, pattern: Vec<Rgba<u8>>) -> Self {
    Pattern {
      insert_index,
      seq: pattern,
      rotation: 0.0,
      reversed: false,
    }
  }
}

impl ColorSequence for Pattern {
  fn base_render(&self, count: usize) -> Vec<image::Rgba<u8>> {
    let mut colors: Vec<Rgba<u8>> = (0..count).map(|_| Rgba::default()).collect();

    for (offset, pixel) in self.seq.iter().enumerate() {
      let index = self.insert_index as usize + offset;

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

  fn reversed(&self) -> bool {
    self.reversed
  }

  fn rotation(&self) -> f32 {
    self.rotation
  }
}
