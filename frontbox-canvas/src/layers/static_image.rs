use std::path::Path;

use crate::{CanvasView, Layer};
use image::RgbaImage;

/// An un-animated image
pub struct StaticImage {
  image: RgbaImage,
}

impl StaticImage {
  pub fn from_path(path: impl AsRef<Path>) -> Self {
    let path = path.as_ref();
    let image =
      image::open(path).unwrap_or_else(|_| panic!("Failed to load static image at {:?}", path));
    Self {
      image: image.into_rgba8(),
    }
  }

  pub fn from_image(image: RgbaImage) -> Self {
    Self { image }
  }

  pub fn width(&self) -> u32 {
    self.image.width()
  }

  pub fn height(&self) -> u32 {
    self.image.height()
  }
}

impl Layer for StaticImage {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    let image_area = self.image.width() * self.image.height();
    let canvas_area = canvas.bounds.width * canvas.bounds.height;

    if image_area < canvas_area {
      for x in 0..self.image.width() {
        for y in 0..self.image.height() {
          canvas.put_pixel(x, y, *self.image.get_pixel(x, y));
        }
      }
    } else {
      for x in 0..canvas.bounds.width {
        for y in 0..canvas.bounds.height {
          canvas.put_pixel(x, y, *self.image.get_pixel(x, y));
        }
      }
    }
  }
}
