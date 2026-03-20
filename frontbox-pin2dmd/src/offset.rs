use std::time::Duration;

use crate::{Renderable, RenderableImage};

pub struct XOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) offset_x: isize,
}

impl Renderable for XOffsetRenderable {
  fn render(&mut self, delta: Duration) -> RenderableImage {
    let mut rendered = self.inner.render(delta);
    rendered.offset_x += self.offset_x;
    rendered
  }
}

pub struct YOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) offset_y: isize,
}

impl Renderable for YOffsetRenderable {
  fn render(&mut self, delta: Duration) -> RenderableImage {
    let mut rendered = self.inner.render(delta);
    rendered.offset_y += self.offset_y;
    rendered
  }
}
