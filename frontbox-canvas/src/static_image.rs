use std::path::Path;

use image::{DynamicImage, GenericImageView};

use crate::{CanvasView, Layer};

/// An un-animated image
pub struct StaticImage {
  image: DynamicImage,
}

impl StaticImage {
  pub fn new(path: impl Into<&'static Path>) -> Self {
    let path = path.into();
    let image =
      image::open(path).unwrap_or_else(|_| panic!("Failed to load static image at {:?}", path));
    Self { image }
  }
}

impl Layer for StaticImage {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    let image_area = self.image.width() * self.image.height();
    let canvas_area = canvas.bounds.width * canvas.bounds.height;

    if image_area < canvas_area {
      for x in 0..self.image.width() {
        for y in 0..self.image.height() {
          canvas.put_pixel(x, y, self.image.get_pixel(x, y));
        }
      }
    } else {
      for x in 0..canvas.bounds.width {
        for y in 0..canvas.bounds.height {
          canvas.put_pixel(x, y, self.image.get_pixel(x, y));
        }
      }
    }
  }
}
