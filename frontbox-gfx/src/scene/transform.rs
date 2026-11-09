use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Shape};

/// Moves, rotates, or scales a node
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees clockwise (y points down), around the node's center as placed by the transforms before this one.
  ///
  /// A `Sequence` has no second axis, so there it scrolls the drawing along the line with the ends wrapped together:
  /// 360 degrees is one full lap.
  Rotate(Prop<f32>),
  Scale(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Lays the node along a traversal of the layer's graph, by name: `(-0.5, 0)` on the node it starts from and
  /// `(0.5, 0)` on the node it visits last, scaled evenly on both axes. A gradient then runs from start to end at
  /// whatever angle the traversal takes.
  ///
  /// Transforms before it are in that frame, where start to end is 1 long, so `[Translate(Fixed(-0.5, 0.0)), Fit("bolt")]`
  /// centers a radial gradient on the start. Changes nothing outside a layer with a graph, or while the traversal visits
  /// nothing or ends where it started.
  Fit(String),
}

impl Transform {
  /// The affine for this step, given where the transforms before it put the node's center
  pub(crate) fn affine(&self, center: Vec2, env: &Env) -> Affine2 {
    match self {
      Self::Translate(offset) => Affine2::from_translation(offset.resolve(env)),
      Self::Rotate(degrees) => match env.lap {
        Some(lap) => Affine2::from_translation(Vec2::new(lap * degrees.resolve(env) / 360.0, 0.0)),
        None => {
          let turn = Affine2::from_angle(degrees.resolve(env).to_radians());
          Affine2::from_translation(center) * turn * Affine2::from_translation(-center)
        }
      },
      Self::Scale(scale) => Affine2::from_scale(scale.resolve(env)),
      Self::Fit(traversal) => {
        let ends = env.visits.and_then(|visits| visits.ends(traversal)).filter(|(start, end)| start.distance(*end) > f32::EPSILON);
        let Some((start, end)) = ends else {
          return Affine2::IDENTITY;
        };
        let along = end - start;
        let fit = Affine2::from_scale_angle_translation(Vec2::splat(along.length()), along.to_angle(), (start + end) / 2.0);
        env.fill_frame * fit
      }
    }
  }

  /// The path of a `Sweep` translate, if this is one
  pub(crate) fn path(&self) -> Option<&Shape> {
    match self {
      Self::Translate(Prop::Sweep(sweep)) => Some(&sweep.path),
      _ => None,
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Translate(prop) | Self::Scale(prop) => prop.is_animated(),
      Self::Rotate(prop) => prop.is_animated(),
      Self::Fit(_) => true,
    }
  }
}
