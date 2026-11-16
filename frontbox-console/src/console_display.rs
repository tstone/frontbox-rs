use frontbox_core::prelude::{ReferencePlane, Vec2};
use frontbox_display::{DisplayMirror, DisplaySize};
use image::Rgb;

use crate::console_hub::ConsoleHub;
use crate::protocol::{DisplayPlacement, DisplayView};

/// Shows a display in the console. Get one from [`WebTracer::display_view`](crate::WebTracer::display_view) and add it
/// to the display with `DisplayDefinition::mirror`. A display without one isn't shown.
pub struct ConsoleDisplay {
  hub: ConsoleHub,
  placement: Option<DisplayPlacement>,
  /// Its place among the hub's displays, once attached
  index: Option<usize>,
}

impl ConsoleDisplay {
  pub(crate) fn new(hub: ConsoleHub) -> Self {
    Self { hub, placement: None, index: None }
  }

  /// Draws it on the machine view, as the rectangle on `plane` from `origin` to `origin + size`, in the plane's units.
  /// Its first row is at the lower y, as a plane's image is, unless the plane stands with its y running up, when it's
  /// turned so its top is up. The plane has to be added to the app. Without this, it's drawn on its own.
  pub fn on(mut self, plane: &ReferencePlane, origin: Vec2, size: Vec2) -> Self {
    self.placement = Some(DisplayPlacement { plane: plane.name.to_string(), origin: origin.to_array(), size: size.to_array() });
    self
  }
}

impl DisplayMirror for ConsoleDisplay {
  fn attach(&mut self, display: &str, size: DisplaySize) {
    let view = DisplayView { name: display.into(), width: size.width, height: size.height, placement: self.placement.take() };
    self.index = Some(self.hub.add_display(view));
  }

  fn show(&mut self, frame: &[Rgb<u8>]) {
    if let Some(index) = self.index {
      self.hub.set_display_frame(index, frame);
    }
  }
}
