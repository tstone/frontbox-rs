use std::time::Duration;

use crate::{Renderable, RenderableImage};

pub struct RecolorSolidRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) color: image::Rgba<u8>,
}

impl Renderable for RecolorSolidRenderable {
  fn render(&self) -> RenderableImage {
    let mut rendered = self.inner.render();
    let mut output = rendered.image.to_rgba8();
    for pixel in output.pixels_mut() {
      let brightness =
        (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0;
      pixel[0] = (self.color[0] as f32 * brightness) as u8;
      pixel[1] = (self.color[1] as f32 * brightness) as u8;
      pixel[2] = (self.color[2] as f32 * brightness) as u8;
    }
    rendered.image = image::DynamicImage::ImageRgba8(output);
    rendered
  }
}

pub struct RecolorVerticalGradientRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) top_color: image::Rgba<u8>,
  pub(crate) bottom_color: image::Rgba<u8>,
}

impl Renderable for RecolorVerticalGradientRenderable {
  fn render(&self) -> RenderableImage {
    let mut rendered = self.inner.render();
    let mut output = rendered.image.to_rgba8();
    let height = output.height() as f32;
    for (y, row) in output.rows_mut().enumerate() {
      let gradient_factor = y as f32 / height;
      let r = self.top_color[0] as f32 * (1.0 - gradient_factor)
        + self.bottom_color[0] as f32 * gradient_factor;
      let g = self.top_color[1] as f32 * (1.0 - gradient_factor)
        + self.bottom_color[1] as f32 * gradient_factor;
      let b = self.top_color[2] as f32 * (1.0 - gradient_factor)
        + self.bottom_color[2] as f32 * gradient_factor;

      for pixel in row {
        let brightness =
          (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0;
        pixel[0] = (r * brightness) as u8;
        pixel[1] = (g * brightness) as u8;
        pixel[2] = (b * brightness) as u8;
      }
    }
    rendered.image = image::DynamicImage::ImageRgba8(output);
    rendered
  }
}

pub struct RecolorHorizontalGradientRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) left_color: image::Rgba<u8>,
  pub(crate) right_color: image::Rgba<u8>,
}

impl Renderable for RecolorHorizontalGradientRenderable {
  fn render(&self) -> RenderableImage {
    let mut rendered = self.inner.render();
    let mut output = rendered.image.to_rgba8();
    let width = output.width() as f32;
    for row in output.rows_mut() {
      for (x, pixel) in row.enumerate() {
        let gradient_factor = x as f32 / width;
        let r = self.left_color[0] as f32 * (1.0 - gradient_factor)
          + self.right_color[0] as f32 * gradient_factor;
        let g = self.left_color[1] as f32 * (1.0 - gradient_factor)
          + self.right_color[1] as f32 * gradient_factor;
        let b = self.left_color[2] as f32 * (1.0 - gradient_factor)
          + self.right_color[2] as f32 * gradient_factor;

        let brightness =
          (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0;
        pixel[0] = (r * brightness) as u8;
        pixel[1] = (g * brightness) as u8;
        pixel[2] = (b * brightness) as u8;
      }
    }
    rendered.image = image::DynamicImage::ImageRgba8(output);
    rendered
  }
}
