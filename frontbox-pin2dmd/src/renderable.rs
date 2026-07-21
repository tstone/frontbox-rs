use image::DynamicImage;

use crate::Asset;
use crate::FrameSize;
use crate::HFlippedRenderable;
use crate::offset::*;
use crate::recolor::*;

pub trait Renderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage;

  /// Flatten the renderable into a single, reusable asset that can be rendered multiple times at low cost
  fn to_asset(&self, parent: &FrameSize) -> Asset {
    let ri = self.render(parent);
    Asset::new(ri.image, ri.offset_x, ri.offset_y)
  }

  /// Offset the image by the specified amount in the x direction
  fn left(self, x: isize) -> LeftOffsetRenderable
  where
    Self: Sized + 'static,
  {
    LeftOffsetRenderable {
      inner: Box::new(self),
      left: x,
    }
  }

  fn right(self, x: isize) -> RightOffsetRenderable
  where
    Self: Sized + 'static,
  {
    RightOffsetRenderable {
      inner: Box::new(self),
      right: x,
    }
  }

  /// Offset the image by the specified amount in the y direction
  fn top(self, y: isize) -> TopOffsetRenderable
  where
    Self: Sized + 'static,
  {
    TopOffsetRenderable {
      inner: Box::new(self),
      top: y,
    }
  }

  fn bottom(self, y: isize) -> BottomOffsetRenderable
  where
    Self: Sized + 'static,
  {
    BottomOffsetRenderable {
      inner: Box::new(self),
      bottom: y,
    }
  }

  /// Offset the image by the specified amount in both x and y directions
  fn offset(self, x: isize, y: isize) -> LeftOffsetRenderable
  where
    Self: Sized + 'static,
  {
    self.top(y).left(x)
  }

  /// Re-color (based on luminance) the image to a single solid color
  fn recolor(self, color: image::Rgba<u8>) -> RecolorSolidRenderable
  where
    Self: Sized + 'static,
  {
    RecolorSolidRenderable {
      inner: Box::new(self),
      color,
    }
  }

  /// Re-color (based on luminance) the image to a vertical gradient between the two colors
  fn recolor_vgradient(
    self,
    top_color: image::Rgba<u8>,
    bottom_color: image::Rgba<u8>,
  ) -> RecolorVerticalGradientRenderable
  where
    Self: Sized + 'static,
  {
    RecolorVerticalGradientRenderable {
      inner: Box::new(self),
      top_color,
      bottom_color,
    }
  }

  /// Re-color (based on luminance) the image to a horizontal gradient between the two colors
  fn recolor_hgradient(
    self,
    left_color: image::Rgba<u8>,
    right_color: image::Rgba<u8>,
  ) -> RecolorHorizontalGradientRenderable
  where
    Self: Sized + 'static,
  {
    RecolorHorizontalGradientRenderable {
      inner: Box::new(self),
      left_color,
      right_color,
    }
  }

  fn flip_h(self) -> HFlippedRenderable
  where
    Self: Sized + 'static,
  {
    HFlippedRenderable {
      inner: Box::new(self),
    }
  }
}

pub struct RenderableImage {
  pub image: DynamicImage,
  pub offset_x: isize,
  pub offset_y: isize,
}

impl RenderableImage {
  pub fn new(image: DynamicImage, x: isize, y: isize) -> Self {
    Self {
      image,
      offset_x: x,
      offset_y: y,
    }
  }
}
