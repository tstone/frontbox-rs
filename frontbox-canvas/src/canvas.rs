use std::{cmp, collections::BTreeMap};

use frontbox::prelude::*;
use image::{ImageBuffer, RgbaImage};

use crate::*;

#[derive(Default)]
pub struct Canvas {
  layers: BTreeMap<i8, Box<dyn PositionedLayer>>,
  highest_layer: i8,
  buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
  size: Size<u32>,
}

impl Canvas {
  pub fn new(width: u32, height: u32) -> Self {
    Self {
      size: Size::new(width, height),
      layers: BTreeMap::new(),
      highest_layer: 0,
      buffer: RgbaImage::new(width, height),
    }
  }

  /// Add a layer above all other layers
  pub fn add(&mut self, layer: impl PositionedLayer + 'static) {
    self.highest_layer += 1;
    self.layers.insert(self.highest_layer, Box::new(layer));
  }

  /// Insert a layer at a specific Z-index
  pub fn insert(&mut self, z_index: i8, layer: impl PositionedLayer + 'static) {
    self.highest_layer = cmp::max(z_index, self.highest_layer);
    self.layers.insert(z_index, Box::new(layer));
  }

  pub fn remove(&mut self, z_index: i8) {
    self.layers.remove(&z_index);
  }

  pub fn clear(&mut self) {
    self.layers.clear();
  }

  pub fn len(&self) -> usize {
    self.layers.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn to_pixels(&mut self) -> Vec<u8> {
    // reset buffer
    for px in self.buffer.pixels_mut() {
      *px = Rgba([0, 0, 0, 0]);
    }

    // render layers
    let mut view = CanvasView {
      buffer: &mut self.buffer,
      origin: Position::zero(),
      bounds: self.size,
    };
    for layer in self.layers.values() {
      layer.render_relative(&mut view);
    }

    // map RGBA to RGB for DMD or LED rendering
    self
      .buffer
      .pixels()
      .flat_map(|p| [p[0], p[1], p[2]])
      .collect()
  }
}
