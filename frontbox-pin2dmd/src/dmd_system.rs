use frontbox::prelude::*;
use frontbox_canvas::*;

use crate::Pin2Dmd;

pub struct DmdSystem {
  dmd: Pin2Dmd,
  canvas: Canvas,
  needs_render: bool,
}

impl DmdSystem {
  pub fn new(dmd: Pin2Dmd) -> Self {
    Self {
      canvas: Canvas::new(dmd.size.width, dmd.size.height),
      dmd,
      needs_render: true,
    }
  }

  fn mark_dirty(&mut self) {
    self.needs_render = true;
  }

  /// Add a layer above all other layers
  pub fn add_layer(&mut self, layer: impl PositionedLayer + 'static) {
    self.canvas.add(layer);
    self.mark_dirty();
  }

  /// Insert a layer at a specific Z-index
  pub fn insert_layer(&mut self, z_index: i8, layer: impl PositionedLayer + 'static) {
    self.canvas.insert(z_index, layer);
    self.mark_dirty();
  }

  pub fn remove_layer(&mut self, z_index: i8) {
    self.canvas.remove(z_index);
    self.mark_dirty();
  }

  pub fn clear(&mut self) {
    self.canvas.clear();
    let _ = self.dmd.clear();
    self.needs_render = false;
  }

  pub fn size(&self) -> &Size<u32> {
    &self.dmd.size
  }
}

impl System for DmdSystem {
  fn on_spawn(&mut self, _ctx: &SystemContext) {
    self.clear();
  }

  fn on_despawn(&mut self, _ctx: &SystemContext) {
    self.clear();
  }

  fn on_render(&mut self, _ctx: &SystemContext) {
    if self.needs_render {
      let start = std::time::Instant::now();

      if !self.canvas.is_empty() {
        log::trace!("canvas layer count: {}", self.canvas.len());
        let pixels = self.canvas.to_pixels();
        let _ = self.dmd.render(&pixels);
      }

      self.needs_render = false;
      log::trace!("DMDSystem on_render took {}μs", start.elapsed().as_micros());
    }
  }
}
