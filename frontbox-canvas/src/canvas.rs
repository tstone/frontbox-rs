use std::{cmp, collections::BTreeMap};

use image::{DynamicImage, RgbaImage};

use crate::*;

#[derive(Default)]
pub struct Canvas {
  layers: BTreeMap<i8, LayerEntry>,
  highest_layer: i8,
}

impl Canvas {
  pub fn new() -> Self {
    Self::default()
  }

  /// Add a layer above all other layers
  pub fn add(&mut self, layer: impl Into<LayerEntry>) {
    self.highest_layer += 1;
    self.layers.insert(self.highest_layer, layer.into());
  }

  /// Insert a layer at a specific Z-index
  pub fn insert(&mut self, z_index: i8, layer: impl Into<LayerEntry>) {
    self.highest_layer = cmp::max(z_index, self.highest_layer);
    self.layers.insert(z_index, layer.into());
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

  pub fn to_image(&self, viewport: &Size<u32>) -> DynamicImage {
    let mut buffer = RgbaImage::new(viewport.width, viewport.height);
    LayerEntry::render_all_at(
      self.layers.values(),
      viewport,
      Position::zero(),
      &mut buffer,
    );
    DynamicImage::ImageRgba8(buffer)
  }

  pub fn to_pixels(&self, viewport: &Size<u32>) -> Vec<u8> {
    self.to_image(viewport).to_rgb8().into_raw()
  }
}
