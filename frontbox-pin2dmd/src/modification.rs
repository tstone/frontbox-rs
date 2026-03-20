use image::DynamicImage;

use crate::{FrameSize, Renderable, RenderableImage};

pub struct HFlippedRenderable {
  pub(crate) inner: Box<dyn Renderable>,
}

impl Renderable for HFlippedRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut rendered = self.inner.render(parent);
    rendered.image = DynamicImage::fliph(&rendered.image);
    rendered
  }
}
