use image::Rgba;

use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GradientStop {
  pub color: Rgba<u8>,
  /// From 0 to 1 along the gradient, or an absolute pixel offset
  pub position: Extent<u16>,
}

impl GradientStop {
  pub fn new(position: impl Into<Extent<u16>>, color: Rgba<u8>) -> Self {
    GradientStop {
      color,
      position: position.into(),
    }
  }
}
