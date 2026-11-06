use glam::Affine2;
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Warp};

/// A function of position, with the changes made to it: warps change the position going in, adjusts change the
/// output coming out
///
/// Shapes and paints are both nodes ([`Shape`](crate::scene::Shape), [`Paint`](crate::scene::Paint)). Anything
/// that changes one is an item in one of these lists, never a new field or wrapper.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<N, A> {
  /// Applied to the drawing in order. Sampling undoes them in reverse on the sample point.
  #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Warp>>", optional))]
  pub warp: Vec<Warp>,
  pub base: N,
  /// Applied to the output in order
  #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Vec<A>>", optional))]
  pub adjust: Vec<A>,
}

impl<N, A> Node<N, A> {
  pub fn new(base: N) -> Self {
    Self { warp: Vec::new(), base, adjust: Vec::new() }
  }

  /// Adds a warp after the existing ones
  pub fn warp(mut self, warp: Warp) -> Self {
    self.warp.push(warp);
    self
  }

  /// Adds an adjust after the existing ones
  pub fn adjust(mut self, adjust: A) -> Self {
    self.adjust.push(adjust);
    self
  }

  /// Maps a point in the scene into the node's own space, undoing its warps. `None` when a warp collapses the drawing (a
  /// scale of zero), so nothing is drawn.
  pub(crate) fn to_local(&self, env: &Env) -> Option<Affine2> {
    let forward = self.warp.iter().fold(Affine2::IDENTITY, |acc, warp| warp.affine(env) * acc);
    if forward.matrix2.determinant().abs() <= f32::EPSILON {
      return None;
    }
    Some(forward.inverse())
  }
}

#[cfg(test)]
mod tests {
  use glam::Vec2;

  use super::*;
  use crate::scene::Params;
  use crate::scene::value::tests::env;

  fn node(warp: Vec<Warp>) -> Node<(), ()> {
    Node { warp, base: (), adjust: vec![] }
  }

  #[test]
  fn warps_apply_to_the_drawing_in_order() {
    let params = Params::new();
    // scale then move: the local point (0.5, 0) is drawn at 2 * 0.5 + 10
    let n = node(vec![
      Warp::Scale(Vec2::splat(2.0).into()),
      Warp::Translate(Vec2::new(10.0, 0.0).into()),
    ]);
    let local = n.to_local(&env(0.0, &params)).unwrap();
    assert!(local.transform_point2(Vec2::new(11.0, 0.0)).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));

    // move then scale: the same point is drawn at 2 * (0.5 + 10)
    let n = node(vec![
      Warp::Translate(Vec2::new(10.0, 0.0).into()),
      Warp::Scale(Vec2::splat(2.0).into()),
    ]);
    let local = n.to_local(&env(0.0, &params)).unwrap();
    assert!(local.transform_point2(Vec2::new(21.0, 0.0)).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));
  }

  #[test]
  fn rotate_turns_x_toward_y() {
    let params = Params::new();
    let n = node(vec![Warp::Rotate(90.0.into())]);
    let local = n.to_local(&env(0.0, &params)).unwrap();
    assert!(local.transform_point2(Vec2::new(0.0, 1.0)).abs_diff_eq(Vec2::new(1.0, 0.0), 1e-6));
  }

  #[test]
  fn zero_scale_draws_nothing() {
    let params = Params::new();
    let n = node(vec![Warp::Scale(Vec2::new(0.0, 1.0).into())]);
    assert!(n.to_local(&env(0.0, &params)).is_none());
  }
}
