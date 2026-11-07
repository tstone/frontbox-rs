use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Transform};

/// A function of position, with the changes made to it: transforms change the position going in, adjusts change the
/// output coming out
///
/// Shapes and fills are both nodes ([`Shape`](crate::scene::Shape), [`Fill`](crate::scene::Fill)). Anything
/// that changes one is an item in one of these lists, never a new field or wrapper.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<N, A> {
  /// Applied to the drawing in order. Sampling undoes them in reverse on the sample point.
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

  /// What each of the transforms does in order, to a drawing whose middle is `center` in its own space
  pub(crate) fn affines(&self, center: Vec2, env: &Env) -> Vec<Affine2> {
    let step = |center: &mut Vec2, transform: &Transform| {
      let affine = transform.affine(*center, env);
      *center = affine.transform_point2(*center);
      Some(affine)
    };
    self.transform.iter().scan(center, step).collect()
  }

  /// Maps a point in the node's own space into the scene, applying its transforms to a drawing whose middle is `center`.
  /// `None` when a transform collapses the drawing (a scale of zero), so nothing is drawn.
  pub(crate) fn to_scene(&self, center: Vec2, env: &Env) -> Option<Affine2> {
    let forward = self.affines(center, env).into_iter().fold(Affine2::IDENTITY, |acc, affine| affine * acc);
    (forward.matrix2.determinant().abs() > f32::EPSILON).then_some(forward)
  }

  /// Where to draw `at` from when `Rotate` runs this node around a `Sequence`: the place that is on the lap nearest the
  /// drawing, whose middle is `center` in its own space, so it's whole wherever its transforms have put it.
  pub(crate) fn on_lap(&self, at: Vec2, center: Vec2, env: &Env) -> Vec2 {
    let Some(lap) = env.lap.filter(|_| self.transform.iter().any(|transform| matches!(transform, Transform::Rotate(_)))) else {
      return at;
    };
    let Some(to_scene) = self.to_scene(center, env) else {
      return at;
    };
    let laps = ((to_scene.transform_point2(center).x - at.x) / lap).round();
    Vec2::new(at.x + laps * lap, at.y)
  }

  /// What maps a point in the scene into the node's own space, undoing its transforms to a drawing whose middle is
  /// `center`. `None` when a transform collapses the drawing.
  pub(crate) fn to_local(&self, center: Vec2, env: &Env) -> Option<Affine2> {
    self.to_scene(center, env).map(|to_scene| to_scene.inverse())
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
      Transform::Scale(Vec2::splat(2.0).into()),
      Transform::Translate(Vec2::new(10.0, 0.0).into()),
    ]);
    let at = Vec2::new(11.0, 0.0);
    let local = n.to_local(Vec2::ZERO, &env(0.0, &params)).unwrap();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));

    // move then scale: the same point is drawn at 2 * (0.5 + 10)
    let n = node(vec![
      Transform::Translate(Vec2::new(10.0, 0.0).into()),
      Transform::Scale(Vec2::splat(2.0).into()),
    ]);
    let at = Vec2::new(21.0, 0.0);
    let local = n.to_local(Vec2::ZERO, &env(0.0, &params)).unwrap();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(0.5, 0.0), 1e-6));
  }

  #[test]
  fn rotate_turns_x_toward_y() {
    let params = Params::new();
    let n = node(vec![Transform::Rotate(90.0.into())]);
    let at = Vec2::new(0.0, 1.0);
    let local = n.to_local(Vec2::ZERO, &env(0.0, &params)).unwrap();
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
    let local = n.to_local(center, &env(0.0, &params)).unwrap();
    assert!(local.transform_point2(at).abs_diff_eq(Vec2::new(2.0, 0.0), 1e-5));
  }

  #[test]
  fn zero_scale_draws_nothing() {
    let params = Params::new();
    let n = node(vec![Transform::Scale(Vec2::new(0.0, 1.0).into())]);
    assert!(n.to_local(Vec2::ZERO, &env(0.0, &params)).is_none());
  }
}
