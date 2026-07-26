use frontbox::prelude::{Rgba, RgbaColor};

use crate::*;

pub struct RecolorLayer {
  pub color: Rgba<u8>,
  pub other: Box<dyn Layer>,
}

impl Layer for RecolorLayer {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    let mut recolor_buffer = RecolorPixelBuffer {
      color: self.color,
      underlying: canvas.buffer,
    };
    let mut recolor_canvas = CanvasView {
      buffer: &mut recolor_buffer,
      origin: canvas.origin.clone(),
      bounds: canvas.bounds.clone(),
    };
    self.other.render(&mut recolor_canvas);
  }
}

struct RecolorPixelBuffer<'a> {
  color: Rgba<u8>,
  underlying: &'a mut dyn PixelBuffer,
}

impl<'a> PixelBuffer for RecolorPixelBuffer<'a> {
  fn get_pixel_at(&mut self, x: u32, y: u32) -> &Rgba<u8> {
    self.underlying.get_pixel_at(x, y)
  }

  fn put_pixel_at(&mut self, x: u32, y: u32, pixel: Rgba<u8>) {
    let brightness = pixel.brightness();
    let r = (self.color[0] as f32 * brightness) as u8;
    let g = (self.color[1] as f32 * brightness) as u8;
    let b = (self.color[2] as f32 * brightness) as u8;

    self
      .underlying
      .put_pixel_at(x, y, Rgba([r, g, b, pixel[3]]));
  }
}
