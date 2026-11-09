use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Fill, Bounds, Mask, Sample, Shape};

/// A fill drawn in a shape
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
  /// Display name for editors and error messages. Doesn't need to be unique.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub name: Option<String>,
  /// Skipped when drawing, but still validated by `check`
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub hidden: bool,
  pub fill: Fill,
  /// `None` covers the whole viewport
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub shape: Option<Shape>,
  /// `None` shows the whole layer
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub mask: Option<Mask>,
}

impl Layer {
  pub fn new(fill: Fill) -> Self {
    Self { name: None, hidden: false, fill, shape: None, mask: None }
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }

  pub fn hidden(mut self) -> Self {
    self.hidden = true;
    self
  }

  pub fn shape(mut self, shape: Shape) -> Self {
    self.shape = Some(shape);
    self
  }

  pub fn mask(mut self, mask: Mask) -> Self {
    self.mask = Some(mask);
    self
  }

  /// How logs and errors refer to the layer at `index`: its name, or else its 1-based position from the bottom (as the
  /// console numbers them)
  pub fn label(&self, index: usize) -> String {
    match &self.name {
      Some(name) => format!("layer \"{name}\""),
      None => format!("layer {}", index + 1),
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.fill.is_animated()
      || self.shape.as_ref().is_some_and(Shape::is_animated)
      || self.mask.as_ref().is_some_and(Mask::is_animated)
  }

  /// Names of the images the layer and its mask use
  pub(crate) fn image_names(&self) -> impl Iterator<Item = &str> {
    let masked = self.mask.as_ref().and_then(|mask| mask.fill.base.image_name());
    self.fill.base.image_name().into_iter().chain(masked)
  }

  pub(crate) fn sample(&self, at: &Sample, viewport: Bounds, env: &Env) -> Rgba<u8> {
    let mut color = sample_fill(&self.fill, self.shape.as_ref(), at, viewport, env);
    if let Some(mask) = self.mask.as_ref().filter(|_| color[3] > 0) {
      color[3] = (color[3] as f32 * mask.amount(at, viewport, env)).round() as u8;
    }
    color
  }

  pub(crate) fn outline(&self, viewport: Bounds, env: &Env) -> Vec<Vec2> {
    fill_outline(self.shape.as_ref(), viewport, env)
  }
}

/// The fill's color at a sample, clipped to the shape (or the viewport, with no shape). Edge coverage goes into alpha.
pub(crate) fn sample_fill(fill: &Fill, shape: Option<&Shape>, at: &Sample, viewport: Bounds, env: &Env) -> Rgba<u8> {
  let (coverage, fill_pos, footprint) = match shape {
    None if !viewport.contains(at.pos) => return Rgba([0, 0, 0, 0]),
    None => (1.0, viewport.to_unit(at.pos), viewport.to_unit_length(at.radius)),
    Some(shape) => match shape.cover(at, env) {
      Some((coverage, local, reach)) => {
        let bounds = shape.base.bounds(env);
        (coverage, bounds.to_unit(local), reach * bounds.to_unit_length(1.0))
      }
      None => return Rgba([0, 0, 0, 0]),
    },
  };

  let mut color = fill.color(fill_pos, footprint, env);
  color[3] = (color[3] as f32 * coverage).round() as u8;
  color
}

/// The area a fill is drawn in, as a closed polygon in scene units
pub(crate) fn fill_outline(shape: Option<&Shape>, viewport: Bounds, env: &Env) -> Vec<Vec2> {
  match shape {
    None => {
      let Bounds { min, max } = viewport;
      vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)]
    }
    Some(shape) => shape.outline(env),
  }
}
