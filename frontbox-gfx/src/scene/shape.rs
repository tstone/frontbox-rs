use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Node, Prop, Bounds, Sample, Warp};

/// How many points an ellipse's outline has. Enough that it looks round on screen.
const ELLIPSE_OUTLINE_POINTS: usize = 64;

/// Where a layer is drawn
pub type Shape = Node<ShapeNode, ShapeAdjust>;

/// Shapes have no position or size of their own; warps give them that. A circle of radius 20 is an `Ellipse`
/// scaled by `(40, 40)`, and a rect is a unit square polygon scaled to its size.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeNode {
  /// An ellipse 1 unit across, centered on the origin
  Ellipse,
  /// A closed polygon through points
  Polygon {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
}

/// Changes to a shape's edge. None yet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeAdjust {}

impl Shape {
  /// How much of the sample's footprint the shape covers (0 to 1), and the sample's position in the shape's own
  /// space. `None` when the shape draws nothing.
  pub(crate) fn cover(&self, at: &Sample, env: &Env) -> Option<(f32, Vec2)> {
    let to_local = self.to_local(env)?;
    let local = to_local.transform_point2(at.pos);
    let (distance, gradient) = self.base.distance(local, env);
    let coverage = coverage(scene_distance(distance, gradient, &to_local), at.radius);
    (coverage > 0.0).then_some((coverage, local))
  }

  /// The shape's edge in the scene, as a closed loop of points. Empty when it draws nothing.
  pub(crate) fn outline(&self, env: &Env) -> Vec<Vec2> {
    let Some(to_scene) = self.to_scene(env) else {
      return Vec::new();
    };
    self.base.outline(env).into_iter().map(|p| to_scene.transform_point2(p)).collect()
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated() || self.warp.iter().any(Warp::is_animated)
  }
}

impl ShapeNode {
  fn is_animated(&self) -> bool {
    match self {
      Self::Ellipse => false,
      Self::Polygon { points } => points.iter().any(Prop::is_animated),
    }
  }

  /// The shape's bounds in its own space, before its warps
  pub(crate) fn bounds(&self, env: &Env) -> Bounds {
    match self {
      Self::Ellipse => Bounds::new(Vec2::splat(-0.5), Vec2::splat(0.5)),
      Self::Polygon { points } => {
        let mut points = points.iter().map(|p| p.resolve(env));
        let Some(first) = points.next() else {
          return Bounds::new(Vec2::ZERO, Vec2::ZERO);
        };
        points.fold(Bounds::new(first, first), |rect, p| Bounds::new(rect.min.min(p), rect.max.max(p)))
      }
    }
  }

  /// The shape's edge in its own space, before its warps
  fn outline(&self, env: &Env) -> Vec<Vec2> {
    match self {
      Self::Ellipse => (0..ELLIPSE_OUTLINE_POINTS)
        .map(|i| Vec2::from_angle(std::f32::consts::TAU * i as f32 / ELLIPSE_OUTLINE_POINTS as f32) * 0.5)
        .collect(),
      // too few points to enclose anything, so it draws nothing
      Self::Polygon { points } if points.len() < 3 => Vec::new(),
      Self::Polygon { points } => points.iter().map(|p| p.resolve(env)).collect(),
    }
  }

  /// Signed distance to the edge (negative inside) in the shape's own space, and the direction it grows in
  fn distance(&self, p: Vec2, env: &Env) -> (f32, Vec2) {
    match self {
      Self::Ellipse => (p.length() - 0.5, p.normalize_or(Vec2::X)),
      Self::Polygon { points } => {
        let points: Vec<Vec2> = points.iter().map(|p| p.resolve(env)).collect();
        polygon_distance(&points, p)
      }
    }
  }
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
  use crate::scene::Params;
  use crate::scene::Warp;
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
      .map_or(0.0, |(c, _)| c)
  }

  #[test]
  fn circle_covers_inside_and_not_outside() {
    let circle = Shape::new(ShapeNode::Ellipse).warp(Warp::Scale(Vec2::splat(10.0).into()));
    assert_eq!(covered(&circle, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 4.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 6.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn edge_through_a_sample_covers_it_partially() {
    let circle = Shape::new(ShapeNode::Ellipse).warp(Warp::Scale(Vec2::splat(10.0).into()));
    assert!((covered(&circle, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&circle, 5.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn ellipse_edge_is_in_scene_units_when_stretched() {
    let ellipse = Shape::new(ShapeNode::Ellipse).warp(Warp::Scale(Vec2::new(20.0, 4.0).into()));
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn square_polygon_covers_its_inside() {
    let rect = Shape::new(square()).warp(Warp::Scale(Vec2::new(10.0, 4.0).into()));
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
  fn zero_radius_is_a_hard_edge() {
    let circle = Shape::new(ShapeNode::Ellipse).warp(Warp::Scale(Vec2::splat(10.0).into()));
    assert_eq!(covered(&circle, 4.99, 0.0, 0.0), 1.0);
    assert_eq!(covered(&circle, 5.01, 0.0, 0.0), 0.0);
  }
}
