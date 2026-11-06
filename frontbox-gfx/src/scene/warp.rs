use glam::{Affine2, Mat2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop};

/// Changes where a node is drawn
///
/// `Mirror` and `Repeat` work around the origin, so a later `Translate` moves the result as a whole.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Warp {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees, clockwise in a scene whose y runs down
  Rotate(Prop<f32>),
  Scale(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Folds the drawing across a line through the origin, replacing what's on the far side with a reflection of what's
  /// on the near side. At 0 degrees the line runs along y and the near side is positive x. Degrees turn it as `Rotate`
  /// does.
  Mirror(Prop<f32>),
  /// Copies of the drawing this far apart on each axis, without end. An axis of 0 doesn't repeat. Each copy is the part
  /// of the drawing in a cell this size centered on the origin, so a drawing that's bigger gets cut off.
  Repeat(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
}

impl Warp {
  /// What this does to a point of the drawing. `Mirror` and `Repeat` differ from place to place, so for them this is the
  /// drawing they start from.
  pub(crate) fn affine(&self, env: &Env) -> Affine2 {
    match self {
      Self::Translate(offset) => Affine2::from_translation(offset.resolve(env)),
      Self::Rotate(degrees) => Affine2::from_angle(degrees.resolve(env).to_radians()),
      Self::Scale(scale) => Affine2::from_scale(scale.resolve(env)),
      Self::Mirror(_) | Self::Repeat(_) => Affine2::IDENTITY,
    }
  }

  /// What takes the point `p` back to where it was before this warp. `None` when this collapses the drawing.
  pub(crate) fn undo(&self, p: Vec2, env: &Env) -> Option<Affine2> {
    match self {
      Self::Mirror(degrees) => {
        let normal = Vec2::from_angle(degrees.resolve(env).to_radians());
        Some(if p.dot(normal) < 0.0 { reflection(normal) } else { Affine2::IDENTITY })
      }
      Self::Repeat(spacing) => {
        let spacing = spacing.resolve(env);
        Some(Affine2::from_translation(-Vec2::new(nearest_copy(p.x, spacing.x), nearest_copy(p.y, spacing.y))))
      }
      _ => {
        let forward = self.affine(env);
        (forward.matrix2.determinant().abs() > f32::EPSILON).then(|| forward.inverse())
      }
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Translate(prop) | Self::Scale(prop) | Self::Repeat(prop) => prop.is_animated(),
      Self::Rotate(prop) | Self::Mirror(prop) => prop.is_animated(),
    }
  }
}

/// Reflects across the line through the origin at right angles to `normal`
fn reflection(normal: Vec2) -> Affine2 {
  let flip = Mat2::from_cols(normal * normal.x, normal * normal.y) * 2.0;
  Affine2::from_mat2(Mat2::IDENTITY - flip)
}

/// How far along one axis the copy nearest to `position` was moved
fn nearest_copy(position: f32, spacing: f32) -> f32 {
  if spacing == 0.0 { 0.0 } else { (position / spacing).round() * spacing }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Params, Track};

  fn back(warp: Warp, p: Vec2) -> Vec2 {
    let params = Params::new();
    warp.undo(p, &env(0.0, &params)).unwrap().transform_point2(p)
  }

  #[test]
  fn mirror_folds_the_far_side_onto_the_near_side() {
    let mirror = Warp::Mirror(0.0.into());
    assert_eq!(back(mirror.clone(), Vec2::new(3.0, 4.0)), Vec2::new(3.0, 4.0));
    assert_eq!(back(mirror, Vec2::new(-3.0, 4.0)), Vec2::new(3.0, 4.0));
  }

  #[test]
  fn mirror_turns_with_its_degrees() {
    let mirror = Warp::Mirror(90.0.into());
    assert!(back(mirror.clone(), Vec2::new(3.0, 4.0)).abs_diff_eq(Vec2::new(3.0, 4.0), 1e-6));
    assert!(back(mirror, Vec2::new(3.0, -4.0)).abs_diff_eq(Vec2::new(3.0, 4.0), 1e-6));
  }

  #[test]
  fn repeat_brings_a_point_into_the_cell_around_the_origin() {
    let repeat = Warp::Repeat(Vec2::new(10.0, 4.0).into());
    assert_eq!(back(repeat.clone(), Vec2::new(23.0, 1.0)), Vec2::new(3.0, 1.0));
    assert_eq!(back(repeat.clone(), Vec2::new(-23.0, 3.0)), Vec2::new(-3.0, -1.0));
    assert_eq!(back(repeat, Vec2::new(4.0, -1.0)), Vec2::new(4.0, -1.0));
  }

  #[test]
  fn repeat_leaves_an_axis_of_zero_alone() {
    let repeat = Warp::Repeat(Vec2::new(10.0, 0.0).into());
    assert_eq!(back(repeat, Vec2::new(23.0, 99.0)), Vec2::new(3.0, 99.0));
  }

  #[test]
  fn folds_change_with_their_props() {
    let track = Track { keys: vec![], cycle: Default::default() };
    assert!(Warp::Mirror(Prop::Track(track)).is_animated());
    assert!(!Warp::Repeat(Vec2::ONE.into()).is_animated());
  }
}
