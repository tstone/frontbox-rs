use std::sync::mpsc;

use frontbox_core::prelude::*;
use frontbox_gfx::canvas::*;
use image::Rgb;

use crate::DmdDevice;
use crate::dmd_manager::DmdManager;

pub struct DmdSystem {
  size: Size<u32>,
  canvas: Canvas,
  needs_render: bool,
  tx: mpsc::Sender<Vec<Rgb<u8>>>,
  /// The manager, until the system spawns and starts it on its own thread
  manager: Option<DmdManager>,
  /// The last frame sent, so an unchanged one isn't sent again
  last_frame: Vec<Rgb<u8>>,
}

impl DmdSystem {
  /// Show frames on `device`. On a hardware platform it's opened once the system spawns, and until it opens, and whenever
  /// it's lost after, it's tried again with a growing wait. On a virtual platform the frames go nowhere for now.
  pub fn new(device: impl DmdDevice) -> Self {
    let size = device.size();
    let (tx, rx) = mpsc::channel();
    Self {
      size,
      canvas: Canvas::new(size.width, size.height),
      needs_render: true,
      tx,
      manager: Some(DmdManager::new(Box::new(device), rx)),
      last_frame: Vec::new(),
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
    self.mark_dirty();
  }

  pub fn size(&self) -> &Size<u32> {
    &self.size
  }

  fn render(&mut self) {
    let frame: Vec<Rgb<u8>> = if self.canvas.is_empty() {
      vec![Rgb([0, 0, 0]); self.size.width as usize * self.size.height as usize]
    } else {
      log::trace!(target: "frontbox::dmd", "canvas layer count: {}", self.canvas.len());
      self
        .canvas
        .to_pixels()
        .chunks_exact(3)
        .map(|rgb| Rgb([rgb[0], rgb[1], rgb[2]]))
        .collect()
    };
    if frame != self.last_frame {
      let _ = self.tx.send(frame.clone());
      self.last_frame = frame;
    }
  }
}

impl System for DmdSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.clear();
    let Some(manager) = self.manager.take() else {
      return;
    };
    if ctx.platform == PlatformKind::Virtual {
      log::info!(target: "frontbox::dmd", "🖥️ On a virtual platform, so the DMD isn't opened");
      return;
    }
    if let Err(e) = std::thread::Builder::new()
      .name("dmd".into())
      .spawn(move || manager.run())
    {
      log::error!(target: "frontbox::dmd", "🖥️ Couldn't start the DMD thread: {}", e);
    }
  }

  fn on_despawn(&mut self, _ctx: &SystemContext) {
    self.clear();
    self.render();
  }

  fn on_render(&mut self, _ctx: &SystemContext) {
    if self.needs_render {
      let start = std::time::Instant::now();
      self.render();
      self.needs_render = false;
      log::trace!(target: "frontbox::dmd", "DMDSystem on_render took {}μs", start.elapsed().as_micros());
    }
  }
}
