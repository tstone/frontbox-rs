use frontbox::prelude::*;
use frontbox_canvas::*;

use crate::Pin2Dmd;

pub struct DmdSystem {
  dmd: Pin2Dmd,
  canvas: Canvas,
}

impl DmdSystem {
  pub fn new(dmd: Pin2Dmd) -> Self {
    Self {
      dmd,
      canvas: Canvas::new(),
    }
  }

  /// Add a layer above all other layers
  pub fn add_layer(&mut self, layer: impl Into<LayerEntry>) {
    self.canvas.add(layer.into());
  }

  /// Insert a layer at a specific Z-index
  pub fn insert_layer(&mut self, z_index: i8, layer: impl Into<LayerEntry>) {
    self.canvas.insert(z_index, layer.into());
  }

  pub fn remove_layer(&mut self, z_index: i8) {
    self.canvas.remove(z_index);
  }

  pub fn clear(&mut self) {
    self.canvas.clear();
    let _ = self.dmd.clear();
  }

  pub fn size(&self) -> &Size<u32> {
    &self.dmd.size
  }
}

impl System for DmdSystem {
  fn on_spawn(&mut self, _ctx: &Context) {
    self.clear();
  }

  fn on_render(&mut self, _ctx: &Context) {
    let start = std::time::Instant::now();
    if self.canvas.len() > 0 {
      log::trace!("canvas layer count: {}", self.canvas.len());
      let pixels = self.canvas.to_pixels(&self.dmd.size);
      let _ = self.dmd.render(&pixels);
    }
    log::trace!(
      "DMDSystem on_render elapsed {}",
      start.elapsed().as_micros()
    );
  }
}
