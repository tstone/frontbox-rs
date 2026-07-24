use image::{DynamicImage, RgbaImage};

use crate::*;

#[derive(Default)]
pub struct Canvas {
  layers: Vec<LayerEntry>,
}

impl Canvas {
  pub fn new() -> Self {
    Self::default()
  }

  /// Add a layer above all other layers
  pub fn push(&mut self, layer: impl Into<LayerEntry>) {
    self.layers.push(layer.into());
  }

  /// Insert a layer at a specific index, shifting all layers after it rightward
  pub fn insert(&mut self, index: usize, layer: impl Into<LayerEntry>) {
    self.layers.insert(index, layer.into());
  }

  pub fn remove(&mut self, index: usize) {
    self.layers.remove(index);
  }

  pub fn clear(&mut self) {
    self.layers.clear();
  }

  pub fn to_image(&self, viewport: &Size<u32>) -> DynamicImage {
    let mut buffer = RgbaImage::new(viewport.width, viewport.height);
    LayerEntry::render_all_at(&self.layers, viewport, Position::zero(), &mut buffer);
    DynamicImage::ImageRgba8(buffer)
  }

  pub fn to_pixels(&self, viewport: &Size<u32>) -> Vec<u8> {
    self.to_image(viewport).to_rgba8().into_raw()
  }
}
