use std::sync::Arc;

use image::DynamicImage;

use crate::{FrameSize, Renderable, RenderableImage};

/// An asset holds a pre-rendered or pre-loaded image that can be cheaply reused across multiple frames
#[derive(Clone)]
pub struct Asset {
  img: Arc<DynamicImage>,
  offset_x: isize,
  offset_y: isize,
}

impl Asset {
  pub fn new(img: DynamicImage, x: isize, y: isize) -> Self {
    Self {
      img: Arc::new(img),
      offset_x: x,
      offset_y: y,
    }
  }

  pub fn image(img: DynamicImage) -> Self {
    Self {
      img: Arc::new(img),
      offset_x: 0,
      offset_y: 0,
    }
  }

  pub fn from_path(path: String) -> Self {
    let img = image::open(&path).unwrap_or_else(|_| panic!("Failed to load asset at {}", path));
    Self::image(img)
  }
}

impl Renderable for Asset {
  fn render(&self, _parent: &FrameSize) -> crate::RenderableImage {
    RenderableImage::new((*self.img).clone(), self.offset_x, self.offset_y)
  }
}
