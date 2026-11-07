use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Node, Prop, Bounds, Sample, Transform};

/// How many points an ellipse's outline has. Enough that it looks round on screen.
const ELLIPSE_OUTLINE_POINTS: usize = 64;

/// Where a layer is drawn
pub type Shape = Node<ShapeNode, ShapeAdjust>;

/// Shapes sit around the origin; transforms put them in the scene. The sizes here are in the shape's own space, so a
/// `Scale` multiplies them.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeNode {
  /// An ellipse centered on the origin, `r1` up and down from its center and `r2` side to side
  Ellipse { r1: Prop<f32>, r2: Prop<f32> },
  /// A rectangle centered on the origin
  Rect { width: Prop<f32>, height: Prop<f32> },
  /// A closed polygon through points
  Polygon {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
}

/// A shape as it stands at a moment, for an editor to draw over
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePose {
  /// What each of the shape's transforms does, in the order they apply
  pub transforms: Vec<Affine2>,
  /// The shape's bounds in its own space, before its transforms
  pub bounds: Bounds,
  /// A polygon's points in its own space. Empty for the other shapes.
  pub points: Vec<Vec2>,
  /// The shape's edge in the scene, as a closed loop of points
  pub outline: Vec<Vec2>,
  /// For each of the shape's transforms, the path it moves the shape along, if it does and that path has a pose. Its
  /// scene is the values the transform takes.
  pub paths: Vec<Option<ShapePose>>,
}

/// Changes to a shape's edge. None yet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeAdjust {}

impl Shape {
  /// How much of the sample's footprint the shape covers (0 to 1), and the sample's position in the shape's own
  /// space. `None` when the shape draws nothing.
  pub(crate) fn cover(&self, at: &Sample, env: &Env) -> Option<(f32, Vec2, Vec2)> {
    let center = self.base.bounds(env).center();
    let pos = self.on_lap(at.pos, center, env);
    let to_local = self.to_local(center, env)?;
    let local = to_local.transform_point2(pos);
    let (distance, gradient) = self.base.distance(local, env);
    let coverage = coverage(scene_distance(distance, gradient, &to_local), at.radius);
    (coverage > 0.0).then(|| (coverage, local, to_local.matrix2.abs() * Vec2::splat(at.radius)))
  }

  /// The shape's edge in the scene, as a closed loop of points. Empty when it draws nothing.
  pub(crate) fn outline(&self, env: &Env) -> Vec<Vec2> {
    let Some(to_scene) = self.to_scene(self.base.bounds(env).center(), env) else {
      return Vec::new();
    };
    self.base.outline(env).into_iter().map(|p| to_scene.transform_point2(p)).collect()
  }

  /// The shape as it stands at a moment. `None` when a transform collapses it.
  pub(crate) fn pose(&self, env: &Env) -> Option<ShapePose> {
    let bounds = self.base.bounds(env);
    self.to_scene(bounds.center(), env)?;
    let points = match &self.base {
      ShapeNode::Polygon { points } => points.iter().map(|p| p.resolve(env)).collect(),
      _ => Vec::new(),
    };
    Some(ShapePose {
      transforms: self.affines(bounds.center(), env),
      bounds,
      points,
      outline: self.outline(env),
      paths: self.transform.iter().map(|transform| transform.path().and_then(|path| path.pose(env))).collect(),
    })
  }

  /// The point in the scene a fraction of the way around the shape's edge. The origin when it has no edge to go around.
  pub(crate) fn along(&self, phase: f32, env: &Env) -> Vec2 {
    match self.to_scene(self.base.bounds(env).center(), env) {
      Some(to_scene) => to_scene.transform_point2(self.base.along(phase, env)),
      None => Vec2::ZERO,
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated() || self.transform.iter().any(Transform::is_animated)
  }
}

impl ShapeNode {
  /// An `Ellipse` with both radii the same
  pub fn circle(radius: f32) -> Self {
    Self::Ellipse { r1: radius.into(), r2: radius.into() }
  }

  fn is_animated(&self) -> bool {
    match self {
      Self::Ellipse { r1: a, r2: b } | Self::Rect { width: a, height: b } => a.is_animated() || b.is_animated(),
      Self::Polygon { points } => points.iter().any(Prop::is_animated),
    }
  }

  /// The shape's bounds in its own space, before its transforms
  pub(crate) fn bounds(&self, env: &Env) -> Bounds {
    match self {
      Self::Ellipse { .. } | Self::Rect { .. } => {
        let reach = self.reach(env);
        Bounds::new(-reach, reach)
      }
      Self::Polygon { points } => {
        let mut points = points.iter().map(|p| p.resolve(env));
        let Some(first) = points.next() else {
          return Bounds::new(Vec2::ZERO, Vec2::ZERO);
        };
        points.fold(Bounds::new(first, first), |rect, p| Bounds::new(rect.min.min(p), rect.max.max(p)))
      }
    }
  }

  /// The shape's edge in its own space, before its transforms
  fn outline(&self, env: &Env) -> Vec<Vec2> {
    match self {
      Self::Ellipse { .. } => (0..ELLIPSE_OUTLINE_POINTS)
        .map(|i| Vec2::from_angle(std::f32::consts::TAU * i as f32 / ELLIPSE_OUTLINE_POINTS as f32) * self.reach(env))
        .collect(),
      Self::Rect { .. } => {
        let reach = self.reach(env);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2::new(x, y) * reach).to_vec()
      }
      // too few points to enclose anything, so it draws nothing
      Self::Polygon { points } if points.len() < 3 => Vec::new(),
      Self::Polygon { points } => points.iter().map(|p| p.resolve(env)).collect(),
    }
  }

  /// The point a fraction of the way around the shape's edge, in its own space. An ellipse needs no outline for this, as
  /// a prop can resolve once for every point being drawn.
  fn along(&self, phase: f32, env: &Env) -> Vec2 {
    match self {
      Self::Ellipse { .. } => Vec2::from_angle(std::f32::consts::TAU * phase) * self.reach(env),
      _ => along_loop(&self.outline(env), phase),
    }
  }

  /// How far an ellipse or a rect reaches from its center along each axis
  fn reach(&self, env: &Env) -> Vec2 {
    match self {
      Self::Ellipse { r1, r2 } => Vec2::new(r2.resolve(env), r1.resolve(env)),
      Self::Rect { width, height } => Vec2::new(width.resolve(env), height.resolve(env)) / 2.0,
      Self::Polygon { .. } => Vec2::ZERO,
    }
  }

  /// Signed distance to the edge (negative inside) in the shape's own space, and the direction it grows in
  fn distance(&self, p: Vec2, env: &Env) -> (f32, Vec2) {
    match self {
      Self::Ellipse { .. } => ellipse_distance(self.reach(env), p),
      Self::Rect { .. } => rect_distance(self.reach(env), p),
      Self::Polygon { points } => {
        let points: Vec<Vec2> = points.iter().map(|p| p.resolve(env)).collect();
        polygon_distance(&points, p)
      }
    }
  }
}

/// The point a fraction of the distance around a closed loop of points
fn along_loop(points: &[Vec2], phase: f32) -> Vec2 {
  let edges = || points.iter().zip(points.iter().cycle().skip(1));
  let mut left = phase * edges().map(|(a, b)| a.distance(*b)).sum::<f32>();
  for (a, b) in edges() {
    let length = a.distance(*b);
    if left <= length && length > 0.0 {
      return a.lerp(*b, left / length);
    }
    left -= length;
  }
  points.first().copied().unwrap_or_default()
}

/// Converts a distance in the shape's own space to scene units. Exact under uniform scale, and close to the edge
/// under non-uniform scale, which is where coverage needs it.
fn scene_distance(distance: f32, gradient: Vec2, to_local: &Affine2) -> f32 {
  let stretch = (to_local.matrix2.transpose() * gradient).length();
  if stretch > f32::EPSILON { distance / stretch } else { distance }
}

/// A linear ramp across the sample's footprint: a sample centered on the edge is half covered
fn coverage(distance: f32, radius: f32) -> f32 {
  if radius <= 0.0 {
    return if distance <= 0.0 { 1.0 } else { 0.0 };
  }
  (0.5 - distance / (2.0 * radius)).clamp(0.0, 1.0)
}

/// How far `p` is from the edge of an ellipse as a fraction of its radius there, and how fast that changes. The two
/// together give the distance close to the edge, which is where coverage needs it.
fn ellipse_distance(reach: Vec2, p: Vec2) -> (f32, Vec2) {
  if reach.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let along = p / reach;
  (along.length() - 1.0, along.normalize_or(Vec2::X) / reach)
}

fn rect_distance(reach: Vec2, p: Vec2) -> (f32, Vec2) {
  if reach.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let past = p.abs() - reach;
  let side = Vec2::new(p.x.signum(), p.y.signum());
  if past.max_element() > 0.0 {
    let outside = past.max(Vec2::ZERO);
    return (outside.length(), (outside * side).normalize_or(Vec2::X));
  }
  if past.x > past.y { (past.x, Vec2::X * side.x) } else { (past.y, Vec2::Y * side.y) }
}

fn polygon_distance(points: &[Vec2], p: Vec2) -> (f32, Vec2) {
  if points.len() < 3 {
    return (f32::INFINITY, Vec2::X);
  }

  let mut nearest = p - points[0];
  let mut nearest_sq = nearest.length_squared();
  let mut sign = 1.0;

  let mut j = points.len() - 1;
  for i in 0..points.len() {
    let (a, b) = (points[i], points[j]);
    let edge = b - a;
    let to_p = p - a;

    let length_sq = edge.length_squared();
    let along = if length_sq > 0.0 { (to_p.dot(edge) / length_sq).clamp(0.0, 1.0) } else { 0.0 };
    let offset = to_p - edge * along;
    if offset.length_squared() < nearest_sq {
      nearest = offset;
      nearest_sq = offset.length_squared();
    }

    // flip the sign each time a ray from p crosses an edge
    let crosses = [p.y >= a.y, p.y < b.y, edge.x * to_p.y > edge.y * to_p.x];
    if crosses.iter().all(|c| *c) || crosses.iter().all(|c| !*c) {
      sign = -sign;
    }
    j = i;
  }

  (sign * nearest_sq.sqrt(), sign * nearest.normalize_or(Vec2::X))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::scene::{Key, Params, Track, Transform};
  use crate::scene::value::tests::env;

  fn square() -> ShapeNode {
    ShapeNode::Polygon {
      points: [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
        .into_iter()
        .map(|(x, y)| Vec2::new(x, y).into())
        .collect(),
    }
  }

  fn covered(shape: &Shape, x: f32, y: f32, radius: f32) -> f32 {
    let params = Params::new();
    shape
      .cover(&Sample::new(Vec2::new(x, y), radius), &env(0.0, &params))
      .map_or(0.0, |(c, ..)| c)
  }

  #[test]
  fn circle_covers_inside_and_not_outside() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(10.0).into()));
    assert_eq!(covered(&circle, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 4.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 6.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn edge_through_a_sample_covers_it_partially() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(10.0).into()));
    assert!((covered(&circle, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&circle, 5.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn ellipse_edge_is_in_scene_units_when_stretched() {
    let ellipse = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::new(20.0, 4.0).into()));
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn square_polygon_covers_its_inside() {
    let rect = Shape::new(square()).transform(Transform::Scale(Vec2::new(10.0, 4.0).into()));
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&rect, 4.0, 1.5, 0.5), 1.0);
    assert_eq!(covered(&rect, 0.0, 3.0, 0.5), 0.0);
    assert!((covered(&rect, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn polygon_winding_does_not_matter() {
    let ShapeNode::Polygon { mut points } = square() else { unreachable!() };
    points.reverse();
    let shape = Shape::new(ShapeNode::Polygon { points });
    assert_eq!(covered(&shape, 0.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 1.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn polygon_bounds() {
    let params = Params::new();
    let triangle = ShapeNode::Polygon {
      points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(4.0, 1.0).into(), Vec2::new(1.0, 3.0).into()],
    };
    assert_eq!(triangle.bounds(&env(0.0, &params)), Bounds::new(Vec2::ZERO, Vec2::new(4.0, 3.0)));
  }

  #[test]
  fn an_ellipses_radii_are_up_and_down_then_side_to_side() {
    let ellipse = Shape::new(ShapeNode::Ellipse { r1: 2.0.into(), r2: 10.0.into() });
    assert_eq!(covered(&ellipse, 9.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&ellipse, 0.0, 3.0, 0.1), 0.0);
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn an_ellipse_with_no_radius_draws_nothing() {
    let ellipse = Shape::new(ShapeNode::Ellipse { r1: 0.0.into(), r2: 10.0.into() });
    assert_eq!(covered(&ellipse, 0.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn a_rect_is_its_width_and_height_around_the_origin() {
    let rect = Shape::new(ShapeNode::Rect { width: 10.0.into(), height: 4.0.into() });
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&rect, -4.0, 1.5, 0.5), 1.0);
    assert_eq!(covered(&rect, 0.0, 3.0, 0.5), 0.0);
    assert!((covered(&rect, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&rect, 0.0, -2.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn a_rect_edge_is_in_scene_units_when_scaled() {
    let rect = Shape::new(ShapeNode::Rect { width: 1.0.into(), height: 1.0.into() })
      .transform(Transform::Scale(Vec2::new(20.0, 4.0).into()));
    assert!((covered(&rect, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&rect, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
    assert!((covered(&rect, 0.0, 2.25, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_rect_past_a_corner_is_measured_to_the_corner() {
    let rect = Shape::new(ShapeNode::Rect { width: 2.0.into(), height: 2.0.into() });
    assert_eq!(covered(&rect, 1.3, 1.4, 0.2), 0.0);
    assert!(covered(&rect, 1.05, 1.05, 0.2) > 0.0);
  }

  #[test]
  fn a_rect_with_no_width_draws_nothing() {
    let rect = Shape::new(ShapeNode::Rect { width: 0.0.into(), height: 4.0.into() });
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn sizes_change_over_time() {
    let params = Params::new();
    let grows = Prop::Track(Track {
      keys: vec![Key::new(0.0, 2.0, Curve::Linear), Key::new(1.0, 6.0, Curve::Linear)],
      cycle: Cycle::Once,
    });
    let rect = Shape::new(ShapeNode::Rect { width: grows, height: 4.0.into() });
    assert!(rect.is_animated());
    assert_eq!(rect.base.bounds(&env(0.5, &params)), Bounds::new(Vec2::new(-2.0, -2.0), Vec2::new(2.0, 2.0)));
  }

  #[test]
  fn a_pose_has_what_an_editor_needs_to_work_out_a_drag() {
    let params = Params::new();
    let triangle = ShapeNode::Polygon {
      points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(4.0, 0.0).into(), Vec2::new(0.0, 2.0).into()],
    };
    let shape = Shape::new(triangle).transform(Transform::Translate(Vec2::new(1.0, 2.0).into()));
    let pose = shape.pose(&env(0.0, &params)).unwrap();
    assert_eq!(pose.transforms, vec![Affine2::from_translation(Vec2::new(1.0, 2.0))]);
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(4.0, 2.0)));
    assert_eq!(pose.points.len(), 3);
  }

  #[test]
  fn a_pose_turns_around_the_shape_where_it_has_been_moved() {
    let params = Params::new();
    let shape = Shape::new(ShapeNode::circle(1.0))
      .transform(Transform::Translate(Vec2::new(10.0, 0.0).into()))
      .transform(Transform::Rotate(90.0.into()));
    let pose = shape.pose(&env(0.0, &params)).unwrap();
    assert!(pose.transforms[1].transform_point2(Vec2::new(10.0, 0.0)).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-5));
  }

  #[test]
  fn a_collapsed_shape_has_no_pose() {
    let params = Params::new();
    let flat = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Scale(Vec2::ZERO.into()));
    assert_eq!(flat.pose(&env(0.0, &params)), None);
  }

  #[test]
  fn zero_radius_is_a_hard_edge() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(10.0).into()));
    assert_eq!(covered(&circle, 4.99, 0.0, 0.0), 1.0);
    assert_eq!(covered(&circle, 5.01, 0.0, 0.0), 0.0);
  }
}
