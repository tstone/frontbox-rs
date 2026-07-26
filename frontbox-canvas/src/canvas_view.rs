use crate::*;
use frontbox::prelude::*;

/// For performance, a mutable reference to a single RgbaImage is passed around as the root 'canvas'.
/// However this requires all layers to be rendered relative to the whole canvas's area. Instead,
/// CanvasView provides a way to reposition a "view" within the canvas, and write pixels relative
/// to this view rather than the whole canvas.
pub struct CanvasView<'a> {
  pub buffer: &'a mut dyn PixelBuffer,
  // this view's (0,0) in absolute buffer coordinates
  pub origin: Position,
  // this view's local size, for clipping
  pub bounds: Size<u32>,
}

impl<'a> CanvasView<'a> {
  pub fn new(origin: Position, bounds: Size<u32>, buffer: &'a mut impl PixelBuffer) -> Self {
    Self {
      buffer,
      origin,
      bounds,
    }
  }

  pub fn put_pixel(&mut self, x: u32, y: u32, color: Rgba<u8>) {
    if x >= self.bounds.width || y >= self.bounds.height {
      return; // out of this view's local bounds — clipped
    }

    // Remap relative to canvas coordinates
    let x = (self.origin.x + x as i32) as u32;
    let y = (self.origin.y + y as i32) as u32;

    if color[3] == 255 {
      self.buffer.put_pixel_at(x, y, color); // fully opaque, skip blend math
      return;
    }
    let dst = *self.buffer.get_pixel_at(x, y);
    let sa = color[3] as f32 / 255.0;
    let out = |s: u8, d: u8| ((s as f32 * sa) + (d as f32 * (1.0 - sa))) as u8;
    self.buffer.put_pixel_at(
      x,
      y,
      Rgba([
        out(color[0], dst[0]),
        out(color[1], dst[1]),
        out(color[2], dst[2]),
        255,
      ]),
    )
  }

  /// Carve out a child view at a local offset — no allocation, just narrower coordinates
  pub fn child_view(&mut self, offset: Position, size: Size<u32>) -> CanvasView<'_> {
    CanvasView {
      // reborrow the parent's buffer to avoid moving it
      buffer: &mut *self.buffer,
      origin: Position {
        x: self.origin.x + offset.x,
        y: self.origin.y + offset.y,
      },
      // clip to whichever is smaller — child's own size, or what's left of the parent
      bounds: Size {
        width: size
          .width
          .min(self.bounds.width.saturating_sub(offset.x as u32)),
        height: size
          .height
          .min(self.bounds.height.saturating_sub(offset.y as u32)),
      },
    }
  }
}
