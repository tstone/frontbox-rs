use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Paint, Bounds, Sample, Shape};

/// A paint drawn in a shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
  pub paint: Paint,
  /// `None` covers the whole viewport
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub shape: Option<Shape>,
}

impl Layer {
  pub fn new(paint: Paint) -> Self {
    Self { paint, shape: None }
  }

  pub fn shape(mut self, shape: Shape) -> Self {
    self.shape = Some(shape);
    self
  }

  pub(crate) fn sample(&self, at: &Sample, viewport: Bounds, env: &Env) -> Rgba<u8> {
    let (coverage, paint_pos) = match &self.shape {
      None => (1.0, viewport.to_unit(at.pos)),
      Some(shape) => match shape.cover(at, env) {
        Some((coverage, local)) => (coverage, shape.base.bounds(env).to_unit(local)),
        None => return Rgba([0, 0, 0, 0]),
      },
    };

    let mut color = self.paint.color(paint_pos, env);
    color[3] = (color[3] as f32 * coverage).round() as u8;
    color
  }
}
