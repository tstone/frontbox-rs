use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;

use crate::*;

pub struct RecolorGradientLayer {
  pub stops: Vec<GradientStop>,
  pub angle: f32,
  pub other: Box<dyn Layer>,
}

impl Layer for RecolorGradientLayer {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    let mut recolor_buffer = RecolorGradientPixelBuffer {
      gradient: Gradient2d::new(self.stops.clone(), self.angle, canvas.bounds),
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

struct RecolorGradientPixelBuffer<'a> {
  gradient: Gradient2d,
  underlying: &'a mut dyn PixelBuffer,
}

impl<'a> PixelBuffer for RecolorGradientPixelBuffer<'a> {
  fn get_pixel_at(&mut self, x: u32, y: u32) -> &Rgba<u8> {
    self.underlying.get_pixel_at(x, y)
  }

  fn put_pixel_at(&mut self, x: u32, y: u32, pixel: Rgba<u8>) {
    let brightness = pixel.brightness();
    let gradient_pixel = self.gradient.sample_at_point(x, y);

    let r = (gradient_pixel[0] as f32 * brightness) as u8;
    let g = (gradient_pixel[1] as f32 * brightness) as u8;
    let b = (gradient_pixel[2] as f32 * brightness) as u8;

    self
      .underlying
      .put_pixel_at(x, y, Rgba([r, g, b, pixel[3]]));
  }
}
