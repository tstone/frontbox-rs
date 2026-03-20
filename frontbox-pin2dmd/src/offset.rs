use crate::{FrameSize, Renderable, RenderableImage};

pub struct LeftOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) left: isize,
}

impl Renderable for LeftOffsetRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut rendered = self.inner.render(parent);
    rendered.offset_x += self.left;
    rendered
  }
}

pub struct RightOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) right: isize,
}

impl Renderable for RightOffsetRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut rendered = self.inner.render(parent);
    let left = parent.width as isize - (rendered.image.width() as isize + self.right);
    rendered.offset_x += left;
    rendered
  }
}

pub struct TopOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) top: isize,
}

impl Renderable for TopOffsetRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut rendered = self.inner.render(parent);
    rendered.offset_y += self.top;
    rendered
  }
}

pub struct BottomOffsetRenderable {
  pub(crate) inner: Box<dyn Renderable>,
  pub(crate) bottom: isize,
}

impl Renderable for BottomOffsetRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut rendered = self.inner.render(parent);
    let top = parent.height as isize - (rendered.image.height() as isize + self.bottom);
    rendered.offset_y += top;
    rendered
  }
}
