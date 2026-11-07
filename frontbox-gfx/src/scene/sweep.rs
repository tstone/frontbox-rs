use glam::Vec2;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use crate::scene::{Animatable, Env, Shape};

/// A position that goes once around the edge of a shape, or along a route or bezier from one end to the other
///
/// The path is a whole [`Shape`], so its transforms say where it is: a circle moved to a point is an orbit of that point.
/// It starts where the shape's outline does and goes the way that does, which is clockwise in a scene whose y runs down
/// for everything but those made of points, which say. An ellipse goes at an even angle, and the others at an even speed.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sweep {
  pub path: Box<Shape>,
  /// Seconds. It waits at the start of the path until then.
  pub start: f32,
  /// Seconds. It stays at the end of the path from then on, which for all but a route or bezier is back where it started.
  pub end: f32,
}

impl Sweep {
  pub fn new(path: Shape, start: f32, end: f32) -> Self {
    Self { path: Box::new(path), start, end }
  }

  pub(crate) fn resolve(&self, env: &Env) -> Vec2 {
    let span = self.end - self.start;
    let phase = if span > 0.0 { ((env.time - self.start) / span).clamp(0.0, 1.0) } else { 1.0 };
    self.path.along(phase, env)
  }
}

/// Reads the sweep of a `Prop<T>`, refusing one where `T` isn't a position
pub(crate) fn only_for_positions<'de, T: Animatable, D: Deserializer<'de>>(deserializer: D) -> Result<Sweep, D::Error> {
  if T::from_point(Vec2::ZERO).is_none() {
    return Err(D::Error::custom("only a position (Vec2) can be a Sweep"));
  }
  Sweep::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::Cycle;
  use crate::layout::Line;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Fill, FillNode, Frame, Images, Layer, Params, Prop, ShapeNode, Timing, Transform, Viewport};
  use crate::scene::test_scene::Scene;

  fn orbit(path: ShapeNode, start: f32, end: f32) -> Prop<Vec2> {
    let path = Shape::new(path).transform(Transform::Translate(Vec2::new(50.0, 20.0).into()));
    Prop::Sweep(Sweep::new(path, start, end))
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
  fn a_sweep_with_no_length_is_at_its_end() {
    let orbit = orbit(ShapeNode::circle(10.0), 1.0, 1.0);
    assert!(at(&orbit, 0.0).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
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
    let moved = Transform::Translate(Prop::Sweep(Sweep::new(path, 0.0, 2.0)));
    let tall = ShapeNode::Rect { width: 0.1.into(), height: 2.0.into() };
    let layer = Layer::new(Fill::new(FillNode::Solid(Color(image::Rgba([255, 0, 0, 255])).into())));
    let scene = Scene::new(Viewport::Sequence, vec![layer.shape(Shape::new(tall).transform(moved))]);
    let scene = scene.timing(Timing::new(2.0, Cycle::Forever));

    let lit = |time: f32| {
      let params = Params::new();
      let frame = Frame { time, take: 0, params: &params, images: Images::none(), size: Vec2::ZERO, viewport: Viewport::Area };
      let line = Line::new(5);
      line.samples().map(|sample| scene.sample(&sample, &frame)[0] > 0).collect::<Vec<_>>()
    };
    assert_eq!(lit(0.0), [false, false, false, true, false]);
    assert_eq!(lit(1.0), [false, true, false, false, false]);
  }

  #[test]
  fn a_shapes_pose_has_the_paths_it_moves_along() {
    let moved = Transform::Translate(orbit(ShapeNode::circle(10.0), 0.0, 4.0));
    let shape = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Scale(Vec2::ONE.into())).transform(moved);
    let pose = shape.pose(&env(0.0, &Params::new())).unwrap();
    assert!(pose.paths[0].is_none());
    let path = pose.paths[1].as_ref().unwrap();
    assert_eq!(path.bounds.max, Vec2::splat(10.0));
    assert!(path.outline[0].abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
  }

  #[test]
  fn a_path_with_no_edge_stays_at_the_origin() {
    let polygon = ShapeNode::Polygon { points: vec![] };
    assert_eq!(at(&Prop::Sweep(Sweep::new(Shape::new(polygon), 0.0, 1.0)), 0.5), Vec2::ZERO);
  }

  #[test]
  fn a_path_can_be_on_a_sweep_of_its_own() {
    let moon = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(orbit(ShapeNode::circle(10.0), 0.0, 4.0)));
    let moon = Prop::Sweep(Sweep::new(moon, 0.0, 1.0));
    assert!(at(&moon, 0.0).abs_diff_eq(Vec2::new(61.0, 20.0), 1e-4));
    assert!(moon.is_animated());
  }
}
