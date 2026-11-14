use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Transform};

/// A base shape or fill, with transforms that change where it's drawn and adjusts that change its output
///
/// [`Shape`](crate::scene::Shape) and [`Fill`](crate::scene::Fill) are both nodes. New ways of modifying one belong as
/// items in these lists, not as new fields or wrapper types.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<N, A> {
  /// Applied in order. Sampling applies the inverse to the sample point.
  #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Transform>>", optional))]
  pub transform: Vec<Transform>,
  pub base: N,
  /// Applied to the output in order
  #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Vec<A>>", optional))]
  pub adjust: Vec<A>,
}

impl<N, A> Node<N, A> {
  pub fn new(base: N) -> Self {
    Self { transform: Vec::new(), base, adjust: Vec::new() }
  }

  /// Adds a transform after the existing ones
  pub fn transform(mut self, transform: Transform) -> Self {
    self.transform.push(transform);
    self
  }

  /// Adds an adjust after the existing ones
  pub fn adjust(mut self, adjust: A) -> Self {
    self.adjust.push(adjust);
    self
  }

  /// The affine for each transform, in order. `center` is the node's center in local space, which `Rotate` pivots
  /// around after the earlier transforms move it.
  pub(crate) fn affines(&self, center: Vec2, env: &Env) -> Vec<Affine2> {
    let step = |center: &mut Vec2, transform: &Transform| {
      let affine = transform.affine(*center, env);
      *center = affine.transform_point2(*center);
      Some(affine)
    };
    self.transform.iter().scan(center, step).collect()
  }

  /// The combined local-to-scene transform. `None` if it collapses to zero area (such as a zero scale), in which case
  /// nothing is drawn.
  pub(crate) fn to_scene(&self, center: Vec2, env: &Env) -> Option<Affine2> {
    let forward = self.affines(center, env).into_iter().fold(Affine2::IDENTITY, |acc, affine| affine * acc);
    (forward.matrix2.determinant().abs() > f32::EPSILON).then_some(forward)
  }

  /// How a `Sequence` wraps around for a node with a `Rotate`, given its `to_scene`. `None` on an `Area`, or without a
  /// `Rotate`.
  pub(crate) fn wrap(&self, center: Vec2, to_scene: Affine2, env: &Env) -> Option<Wrap> {
    let lap = env.lap.filter(|_| self.transform.iter().any(|transform| matches!(transform, Transform::Rotate(_))))?;
    Some(Wrap { lap, center: to_scene.transform_point2(center).x })
  }
}

/// Where a node on a `Sequence` repeats: once every `lap`, around its `center` along the line
#[derive(Debug, Clone, Copy)]
pub(crate) struct Wrap {
  lap: f32,
  center: f32,
}

impl Wrap {
  /// Shifts `at` by whole laps to whichever copy is nearest the node, so a node scrolled off one end reappears at the
  /// other
  pub(crate) fn apply(&self, at: Vec2) -> Vec2 {
    let laps = ((self.center - at.x) / self.lap).round();
    Vec2::new(at.x + laps * self.lap, at.y)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::Params;
  use crate::scene::value::tests::env;

  fn node(transform: Vec<Transform>) -> Node<(), ()> {
    Node { transform, base: (), adjust: vec![] }
  }

  #[test]
  fn transforms_apply_to_the_drawing_in_order() {
    let params = Params::new();
    // scale then move: the local point (0.5, 0) is drawn at 2 * 0.5 + 10
    let n = node(vec![
      Transform::Scale(2.0.into()),
      Transform::Translate(Vec2::new(10.0, 0.0).into()),
    ]);
    let at = Vec2::new(11.0, 0.0);
    let local = n.to_scene(Vec2::ZERO, &env(0.0, &params)).unwrap().inverse();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));

    // move then scale: the same point is drawn at 2 * (0.5 + 10)
    let n = node(vec![
      Transform::Translate(Vec2::new(10.0, 0.0).into()),
      Transform::Scale(2.0.into()),
    ]);
    let at = Vec2::new(21.0, 0.0);
    let local = n.to_scene(Vec2::ZERO, &env(0.0, &params)).unwrap().inverse();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));
  }

  #[test]
  fn rotate_turns_x_toward_y() {
    let params = Params::new();
    let n = node(vec![Transform::Rotate(90.0.into())]);
    let at = Vec2::new(0.0, 1.0);
    let local = n.to_scene(Vec2::ZERO, &env(0.0, &params)).unwrap().inverse();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(1.0, 0.0), 1e-6));
  }

  #[test]
  fn rotate_turns_around_the_middle_of_a_drawing_wherever_it_has_been_moved() {
    let params = Params::new();
    // the middle stays at (11, 0), and what was 1 past it on x is now 1 past it on y
    let n = node(vec![Transform::Translate(Vec2::new(10.0, 0.0).into()), Transform::Rotate(90.0.into())]);
    let center = Vec2::new(1.0, 0.0);
    let to_scene = n.to_scene(center, &env(0.0, &params)).unwrap();
    assert!(to_scene.transform_point2(center).abs_diff_eq(Vec2::new(11.0, 0.0), 1e-5));
    assert!(to_scene.transform_point2(Vec2::new(2.0, 0.0)).abs_diff_eq(Vec2::new(11.0, 1.0), 1e-5));

    let at = Vec2::new(11.0, 1.0);
    let local = n.to_scene(center, &env(0.0, &params)).unwrap().inverse();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(2.0, 0.0), 1e-5));
  }

  #[test]
  fn zero_scale_draws_nothing() {
    let params = Params::new();
    let n = node(vec![Transform::Scale(0.0.into())]);
    assert!(n.to_scene(Vec2::ZERO, &env(0.0, &params)).is_none());
  }
}
