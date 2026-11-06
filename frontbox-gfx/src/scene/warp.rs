use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop};

/// Changes where a node is drawn
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Warp {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees, clockwise in a scene whose y runs down
  Rotate(Prop<f32>),
  Scale(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
}

impl Warp {
  pub(crate) fn affine(&self, env: &Env) -> Affine2 {
    match self {
      Self::Translate(offset) => Affine2::from_translation(offset.resolve(env)),
      Self::Rotate(degrees) => Affine2::from_angle(degrees.resolve(env).to_radians()),
      Self::Scale(scale) => Affine2::from_scale(scale.resolve(env)),
    }
  }
}
