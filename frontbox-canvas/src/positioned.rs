use std::ops::{Deref, DerefMut};

use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::{Extent, Rgba};

use crate::*;

#[derive(Debug, Clone)]
pub struct Placement {
  pub horizontal: Horizontal,
  pub vertical: Vertical,
  pub width: Extent<u32>,
  pub height: Extent<u32>,
}

impl Default for Placement {
  fn default() -> Self {
    Self {
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      width: Extent::full(),
      height: Extent::full(),
    }
  }
}

pub struct Positioned<L: Layer> {
  pub layer: L,
  pub placement: Placement,
}

impl<L: Layer> Positioned<L> {
  pub fn new(layer: L) -> Self {
    Self {
      placement: Placement::default(),
      layer,
    }
  }

  pub fn layer_mut(&mut self) -> &mut L {
    &mut self.layer
  }

  pub fn horizontal(mut self, h: impl Into<Horizontal>) -> Self {
    self.placement.horizontal = h.into();
    self
  }

  pub fn vertical(mut self, v: impl Into<Vertical>) -> Self {
    self.placement.vertical = v.into();
    self
  }

  pub fn width(mut self, v: impl Into<Extent<u32>>) -> Self {
    self.placement.width = v.into();
    self
  }

  pub fn height(mut self, v: impl Into<Extent<u32>>) -> Self {
    self.placement.height = v.into();
    self
  }

  pub fn recolor(self, color: Rgba<u8>) -> Positioned<RecolorLayer>
  where
    L: 'static,
  {
    Positioned {
      layer: self.layer.recolor(color),
      placement: self.placement.clone(),
    }
  }

  pub fn recolor_fade(
    self,
    from: Rgba<u8>,
    to: Rgba<u8>,
    angle: f32,
  ) -> Positioned<RecolorGradientLayer>
  where
    L: 'static,
  {
    Positioned {
      layer: self.layer.recolor_fade(from, to, angle),
      placement: self.placement.clone(),
    }
  }

  pub fn recolor_gradient(
    self,
    stops: Vec<GradientStop>,
    angle: f32,
  ) -> Positioned<RecolorGradientLayer>
  where
    L: 'static,
  {
    Positioned {
      layer: self.layer.recolor_gradient(stops, angle),
      placement: self.placement.clone(),
    }
  }
}

impl<L: Layer> From<L> for Positioned<L> {
  fn from(value: L) -> Self {
    Positioned {
      layer: value,
      placement: Placement::default(),
    }
  }
}

impl<L: Layer> Deref for Positioned<L> {
  type Target = L;

  fn deref(&self) -> &Self::Target {
    &self.layer
  }
}

impl<L: Layer> DerefMut for Positioned<L> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.layer
  }
}
