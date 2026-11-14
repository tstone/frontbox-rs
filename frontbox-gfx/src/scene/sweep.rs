use glam::Vec2;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use crate::scene::{Animatable, Env, Prop, Shape, ShapeNode};

/// A position on a shape's outline, or along a polyline or bezier
///
/// The path is a full [`Shape`] with its own transforms, so a circle translated to a point becomes an orbit around it.
/// Built-in shapes go clockwise from where their outline starts (3 o'clock for round ones); shapes made of points follow
/// their point order. Circles and ellipses go at a constant angular speed, everything else at a constant speed.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sweep {
  pub path: Box<Shape>,
  /// How far along the path, 0 to 1. Keyframes from 0 to 1 go around once. On a closed shape it carries on past 1,
  /// so going to 2 goes around twice; a polyline or bezier holds at its ends.
  pub position: Box<Prop<f32>>,
}

impl Sweep {
  pub fn new(path: Shape, position: Prop<f32>) -> Self {
    Self { path: Box::new(path), position: Box::new(position) }
  }

  pub(crate) fn resolve(&self, env: &Env) -> Vec2 {
    let position = self.position.resolve(env);
    let position = match self.path.base {
      ShapeNode::Polyline { .. } | ShapeNode::Bezier { .. } => position.clamp(0.0, 1.0),
      _ => position.rem_euclid(1.0),
    };
    self.path.along(position, env)
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.position.is_animated() || self.path.is_animated()
  }
}

/// Deserializes `Prop::Sweep`, rejecting it unless `T` is a position type
pub(crate) fn only_for_positions<'de, T: Animatable, D: Deserializer<'de>>(deserializer: D) -> Result<Sweep, D::Error> {
  if T::from_point(Vec2::ZERO).is_none() {
    return Err(D::Error::custom("only a position (Vec2) can be a Sweep"));
  }
  Sweep::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::layout::Line;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Fill, FillNode, Key, Keyframes, LayerFrame, Assets, Layer, Params, Timing, Transform, Surface, Value, no_params};
  use crate::scene::test_scene::Scene;

  /// Once along the path between `start` and `end` seconds
  fn lap(start: f32, end: f32) -> Prop<f32> {
    Prop::Keyframes(Keyframes::new(vec![Key::new(start, 0.0, Curve::Linear), Key::new(end, 1.0, Curve::Linear)], Cycle::Once))
  }

  fn orbit(path: ShapeNode, start: f32, end: f32) -> Prop<Vec2> {
    let path = Shape::new(path).transform(Transform::Translate(Vec2::new(50.0, 20.0).into()));
    Prop::Sweep(Sweep::new(path, lap(start, end)))
  }

  fn at(prop: &Prop<Vec2>, time: f32) -> Vec2 {
    prop.resolve(&env(time, &Params::new()))
  }

  #[test]
  fn a_circle_orbits_its_center() {
    let orbit = orbit(ShapeNode::circle(10.0), 0.0, 4.0);
    assert!(at(&orbit, 0.0).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&orbit, 1.0).abs_diff_eq(Vec2::new(50.0, 30.0), 1e-4));
    assert!(at(&orbit, 2.0).abs_diff_eq(Vec2::new(40.0, 20.0), 1e-4));
  }

  #[test]
  fn it_waits_for_its_start_and_stays_at_its_end() {
    let orbit = orbit(ShapeNode::circle(10.0), 1.0, 3.0);
    assert!(at(&orbit, 0.0).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&orbit, 2.0).abs_diff_eq(Vec2::new(40.0, 20.0), 1e-4));
    assert!(at(&orbit, 9.0).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
  }

  #[test]
  fn a_closed_path_goes_around_again_past_one() {
    let twice = Keyframes::new(vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(4.0, 2.0, Curve::Linear)], Cycle::Once);
    let twice = Prop::Sweep(Sweep::new(Shape::new(ShapeNode::circle(10.0)), Prop::Keyframes(twice)));
    assert!(at(&twice, 0.5).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
    assert!(at(&twice, 2.5).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
    assert!(at(&twice, 4.0).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4));
  }

  #[test]
  fn a_line_holds_at_its_ends() {
    let line = Shape::new(ShapeNode::Polyline { points: vec![Vec2::ZERO.into(), Vec2::new(10.0, 0.0).into()] });
    let at_position = |position: f32| at(&Prop::Sweep(Sweep::new(line.clone(), position.into())), 0.0);
    assert_eq!(at_position(-0.5), Vec2::ZERO);
    assert_eq!(at_position(1.5), Vec2::new(10.0, 0.0));
  }

  #[test]
  fn its_position_can_come_from_a_param_without_a_timing() {
    let sweep: Prop<Vec2> = Prop::Sweep(Sweep::new(Shape::new(ShapeNode::circle(10.0)), Prop::Param("lap".into())));
    let params = Params::from([("lap".into(), Value::Float(0.5))]);
    assert!(sweep.resolve(&env(0.0, &params)).abs_diff_eq(Vec2::new(-10.0, 0.0), 1e-4));
    assert!(!sweep.is_animated());
  }

  #[test]
  fn a_rect_goes_around_at_an_even_speed() {
    let rect = ShapeNode::Rect { width: 30.0.into(), height: 10.0.into() };
    let around = orbit(rect, 0.0, 8.0);
    // 80 around, starting from the top left corner, so 30 along the top is the top right corner
    assert!(at(&around, 0.0).abs_diff_eq(Vec2::new(35.0, 15.0), 1e-4));
    assert!(at(&around, 3.0).abs_diff_eq(Vec2::new(65.0, 15.0), 1e-4));
    assert!(at(&around, 3.5).abs_diff_eq(Vec2::new(65.0, 20.0), 1e-4));
    assert!(at(&around, 8.0).abs_diff_eq(Vec2::new(35.0, 15.0), 1e-4));
  }

  #[test]
  fn a_polygon_goes_through_its_points_in_order() {
    let points = [Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0), Vec2::new(0.0, 10.0)];
    let polygon = ShapeNode::Polygon { points: points.map(Into::into).to_vec() };
    let around = orbit(polygon, 0.0, 4.0);
    assert!(at(&around, 1.0).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&around, 2.5).abs_diff_eq(Vec2::new(55.0, 30.0), 1e-4));
  }

  #[test]
  fn a_sequence_sees_a_path_as_it_passes_along_the_line() {
    let path = Shape::new(ShapeNode::circle(0.25)).transform(Transform::Translate(Vec2::new(0.5, 0.0).into()));
    let moved = Transform::Translate(Prop::Sweep(Sweep::new(path, lap(0.0, 2.0))));
    let tall = ShapeNode::Rect { width: 0.1.into(), height: 2.0.into() };
    let layer = Layer::new(Fill::new(FillNode::Solid(Color(image::Rgba([255, 0, 0, 255])).into())));
    let scene = Scene::new(Surface::Sequence, vec![layer.region(Shape::new(tall).transform(moved))]);
    let scene = scene.timing(Timing::new(2.0, Cycle::Forever));

    let lit = |time: f32| {
      let params = Params::new();
      let frame = LayerFrame { time, playback_seed: 0, params: &params, assets: Assets::none(), size: Vec2::ZERO, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
      let line = Line::new(5);
      line.samples().map(|sample| scene.sample(&sample, &frame)[0] > 0).collect::<Vec<_>>()
    };
    assert_eq!(lit(0.0), [false, false, false, true, false]);
    assert_eq!(lit(1.0), [false, true, false, false, false]);
  }

  #[test]
  fn a_shapes_pose_has_the_paths_it_moves_along() {
    let moved = Transform::Translate(orbit(ShapeNode::circle(10.0), 0.0, 4.0));
    let shape = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Scale(1.0.into())).transform(moved);
    let pose = shape.pose(&env(0.0, &Params::new())).unwrap();
    assert!(pose.paths[0].is_none());
    let path = pose.paths[1].as_ref().unwrap();
    assert_eq!(path.bounds.max, Vec2::splat(10.0));
    assert!(path.outline[0].abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
  }

  #[test]
  fn a_path_with_no_edge_stays_at_the_origin() {
    let polygon = ShapeNode::Polygon { points: vec![] };
    assert_eq!(at(&Prop::Sweep(Sweep::new(Shape::new(polygon), 0.5.into())), 0.5), Vec2::ZERO);
  }

  #[test]
  fn a_path_can_be_on_a_sweep_of_its_own() {
    let moon = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(orbit(ShapeNode::circle(10.0), 0.0, 4.0)));
    let moon = Prop::Sweep(Sweep::new(moon, 0.0.into()));
    assert!(at(&moon, 0.0).abs_diff_eq(Vec2::new(61.0, 20.0), 1e-4));
    assert!(moon.is_animated());
  }
}
