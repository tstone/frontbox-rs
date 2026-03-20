use image::DynamicImage;

use crate::{Renderable, RenderableImage};

/// A single image
#[derive(Debug, Clone)]
pub struct Sprite;

impl Sprite {
  pub fn path(path: String) -> PathSprite {
    PathSprite { path }
  }

  pub fn image(image: image::DynamicImage) -> ImageSprite {
    ImageSprite { image }
  }
}

/// A single image file
#[derive(Debug, Clone)]
pub struct PathSprite {
  path: String,
}

impl PathSprite {
  pub fn new(path: String) -> Self {
    Self { path }
  }
}

impl Renderable for PathSprite {
  fn render(&self) -> RenderableImage {
    let image =
      image::open(&self.path).unwrap_or_else(|_| panic!("Failed to load sprite at {}", self.path));
    crate::RenderableImage::new(image, 0, 0)
  }
}

/// A single image
#[derive(Debug, Clone)]
pub struct ImageSprite {
  image: image::DynamicImage,
}

impl ImageSprite {
  pub fn new(image: DynamicImage) -> Self {
    Self { image }
  }
}

impl Renderable for ImageSprite {
  fn render(&self) -> RenderableImage {
    crate::RenderableImage::new(self.image.clone(), 0, 0)
  }
}
