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

  pub fn insert_layer(&mut self, z_index: usize, layer: impl Into<LayerEntry>) {
    self.canvas.insert(z_index, layer.into());
  }

  pub fn remove_layer(&mut self, z_index: usize) {
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
  fn on_render(&mut self, _ctx: &Context) {
    let pixels = self.canvas.to_pixels(&self.dmd.size);
    let _ = self.dmd.render(&pixels);
  }
}
