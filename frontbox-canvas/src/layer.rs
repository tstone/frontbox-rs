use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::Rgba;

use crate::*;

/// A layer contains something which can be rendered to a canvas
pub trait Layer {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>);

  fn recolor(self, color: Rgba<u8>) -> RecolorLayer
  where
    Self: Sized + 'static,
  {
    RecolorLayer {
      color,
      other: Box::new(self),
    }
  }

  fn recolor_fade(self, from: Rgba<u8>, to: Rgba<u8>, angle: f32) -> RecolorGradientLayer
  where
    Self: Sized + 'static,
  {
    self.recolor_gradient(
      vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)],
      angle,
    )
  }

  fn recolor_gradient(self, stops: Vec<GradientStop>, angle: f32) -> RecolorGradientLayer
  where
    Self: Sized + 'static,
  {
    RecolorGradientLayer {
      stops,
      angle,
      other: Box::new(self),
    }
  }

  fn left_offset(self, extent: impl Into<Extent<i32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).horizontal(Horizontal::LeftOffset(extent.into()))
  }

  fn right_offset(self, extent: impl Into<Extent<i32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).horizontal(Horizontal::RightOffset(extent.into()))
  }

  fn horizontal(self, h: impl Into<Horizontal>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).horizontal(h.into())
  }

  fn top_offset(self, extent: impl Into<Extent<i32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).vertical(Vertical::TopOffset(extent.into()))
  }

  fn bottom_offset(self, extent: impl Into<Extent<i32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).vertical(Vertical::BottomOffset(extent.into()))
  }

  fn vertical(self, v: impl Into<Vertical>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).vertical(v.into())
  }

  fn width(self, v: impl Into<Extent<u32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).width(v.into())
  }

  fn height(self, v: impl Into<Extent<u32>>) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self).height(v.into())
  }

  fn default_position(self) -> Positioned<Self>
  where
    Self: Sized,
  {
    Positioned::new(self)
  }
}
