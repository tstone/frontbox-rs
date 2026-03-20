use image::DynamicImage;

use crate::offset::*;
use crate::recolor::*;

pub trait Renderable {
  fn render(&self) -> RenderableImage;

  /// Offset the image by the specified amount in the x direction
  fn offset_x(self, x: isize) -> XOffsetRenderable
  where
    Self: Sized + 'static,
  {
    XOffsetRenderable {
      inner: Box::new(self),
      offset_x: x,
    }
  }

  /// Offset the image by the specified amount in the y direction
  fn offset_y(self, y: isize) -> YOffsetRenderable
  where
    Self: Sized + 'static,
  {
    YOffsetRenderable {
      inner: Box::new(self),
      offset_y: y,
    }
  }

  /// Offset the image by the specified amount in both x and y directions
  fn offset(self, x: isize, y: isize) -> XOffsetRenderable
  where
    Self: Sized + 'static,
  {
    self.offset_y(y).offset_x(x)
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
