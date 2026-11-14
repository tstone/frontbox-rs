use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Node, Prop, Bounds, Sample, Transform};

/// Points in the outline polygon of a round shape
const ROUND_OUTLINE_POINTS: usize = 64;

/// Cap on pinwheel and propeller blades, well past where they can be told apart
const MAX_BLADES: f32 = 360.0;

/// Line segments per bezier span when flattening it to a polyline
const BEZIER_STEPS: usize = 24;

/// The area a layer is drawn in
pub type Shape = Node<ShapeNode, ShapeAdjust>;

/// Base shapes, defined around the origin in local space. Transforms place them in the scene.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeNode {
  Circle { radius: Prop<f32> },
  /// An annulus: `thickness` is the band's width outward from the hole. An `inner_radius` of 0 makes a filled circle.
  Ring {
    #[serde(alias = "radius")]
    inner_radius: Prop<f32>,
    thickness: Prop<f32>,
  },
  Ellipse {
    #[serde(alias = "r2")]
    rx: Prop<f32>,
    #[serde(alias = "r1")]
    ry: Prop<f32>,
  },
  Rect { width: Prop<f32>, height: Prop<f32> },
  /// Wedge-shaped blades radiating from the origin out to `radius`.
  ///
  /// Each blade plus the gap after it is an equal slice of the circle, and `blade_fraction` (0 to 1) is the blade's share
  /// of its slice, so at 0.5 blades and gaps are equal. The first blade starts at 3 o'clock and they go clockwise.
  /// `blades` is truncated to a whole number.
  Pinwheel {
    radius: Prop<f32>,
    blades: Prop<f32>,
    #[serde(alias = "balance")]
    blade_fraction: Prop<f32>,
  },
  /// Rectangular blades radiating from the origin out to `radius`, each `blade_width` wide and spaced evenly around the
  /// circle. The first blade points to 3 o'clock and they go clockwise. `blades` is truncated to a whole number.
  Propeller {
    radius: Prop<f32>,
    blades: Prop<f32>,
    blade_width: Prop<f32>,
  },
  /// A closed polygon
  Polygon {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
  /// An open polyline. It has no inside, so it draws nothing as a layer's shape; it's only for a [`Sweep`] to follow.
  ///
  /// [`Sweep`]: crate::scene::Sweep
  #[serde(alias = "Route")]
  Polyline {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
  /// An open, smooth curve through points. Like [`ShapeNode::Polyline`], it's only for a sweep to follow.
  Bezier { points: Vec<BezierPoint> },
}

/// A point a [`ShapeNode::Bezier`] passes through
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierPoint {
  #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
  pub point: Prop<Vec2>,
  /// Tangent handle, relative to `point`. The curve leaves toward `point + handle` and arrives from `point - handle`, so
  /// it's always smooth through the point.
  #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
  pub handle: Prop<Vec2>,
}

/// A snapshot of a shape at one moment, with what an editor needs to draw it and its handles
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePose {
  /// The affine for each transform, in order
  pub transforms: Vec<Affine2>,
  /// Bounds in local space, before transforms
  pub bounds: Bounds,
  /// Local-space points of a polygon, polyline, or bezier. Empty for other shapes.
  pub points: Vec<Vec2>,
  /// Bezier handles, relative to their points. Empty for other shapes.
  pub handles: Vec<Vec2>,
  /// A ring's hole radius in local space. 0 for other shapes.
  pub hole: f32,
  /// Outline in scene space. Closed, except for polylines and beziers. A ring's is its outer edge.
  pub outline: Vec<Vec2>,
  /// For each transform, the pose of the `Sweep` path it follows, if any. The path's coordinates are translate values.
  pub paths: Vec<Option<ShapePose>>,
}

/// Changes to a shape's edge. There are none yet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeAdjust {}

impl Shape {
  /// How much of the sample the shape covers (0 to 1), the sample's position in local space, and its radius in local
  /// space along each axis. `None` if the sample is outside the shape or the shape is collapsed.
  pub(crate) fn cover(&self, at: &Sample, env: &Env) -> Option<(f32, Vec2, Vec2)> {
    let center = self.base.bounds(env).center();
    let pos = self.on_lap(at.pos, center, env);
    let to_local = self.to_local(center, env)?;
    let local = to_local.transform_point2(pos);
    let (distance, gradient) = self.base.distance(local, env);
    let coverage = coverage(scene_distance(distance, gradient, &to_local), at.radius);
    (coverage > 0.0).then(|| (coverage, local, to_local.matrix2.abs() * Vec2::splat(at.radius)))
  }

  /// Outline in scene space. Empty if the shape is collapsed.
  pub(crate) fn outline(&self, env: &Env) -> Vec<Vec2> {
    let Some(to_scene) = self.to_scene(self.base.bounds(env).center(), env) else {
      return Vec::new();
    };
    self.base.outline(env).into_iter().map(|p| to_scene.transform_point2(p)).collect()
  }

  /// `None` if a transform collapses the shape
  pub(crate) fn pose(&self, env: &Env) -> Option<ShapePose> {
    let bounds = self.base.bounds(env);
    self.to_scene(bounds.center(), env)?;
    let points = match &self.base {
      ShapeNode::Polygon { points } | ShapeNode::Polyline { points } => points.iter().map(|p| p.resolve(env)).collect(),
      ShapeNode::Bezier { points } => points.iter().map(|p| p.point.resolve(env)).collect(),
      _ => Vec::new(),
    };
    let handles = match &self.base {
      ShapeNode::Bezier { points } => points.iter().map(|p| p.handle.resolve(env)).collect(),
      _ => Vec::new(),
    };
    let hole = match &self.base {
      ShapeNode::Ring { inner_radius, .. } => inner_radius.resolve(env).max(0.0),
      _ => 0.0,
    };
    Some(ShapePose {
      transforms: self.affines(bounds.center(), env),
      bounds,
      points,
      handles,
      hole,
      outline: self.outline(env),
      paths: self.transform.iter().map(|transform| transform.path().and_then(|path| path.pose(env))).collect(),
    })
  }

  /// The scene-space point `phase` (0 to 1) of the way along the outline. The origin if there's no outline.
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
  pub fn circle(radius: f32) -> Self {
    Self::Circle { radius: radius.into() }
  }

  fn is_animated(&self) -> bool {
    match self {
      Self::Circle { radius } => radius.is_animated(),
      Self::Ring { inner_radius: a, thickness: b }
      | Self::Ellipse { rx: a, ry: b }
      | Self::Rect { width: a, height: b } => {
        a.is_animated() || b.is_animated()
      }
      Self::Pinwheel { radius: a, blades: b, blade_fraction: c } | Self::Propeller { radius: a, blades: b, blade_width: c } => {
        a.is_animated() || b.is_animated() || c.is_animated()
      }
      Self::Polygon { points } | Self::Polyline { points } => points.iter().any(Prop::is_animated),
      Self::Bezier { points } => points.iter().any(|p| p.point.is_animated() || p.handle.is_animated()),
    }
  }

  /// Bounds in local space, before transforms
  pub(crate) fn bounds(&self, env: &Env) -> Bounds {
    match self {
      Self::Circle { .. }
      | Self::Ring { .. }
      | Self::Ellipse { .. }
      | Self::Rect { .. }
      | Self::Pinwheel { .. }
      | Self::Propeller { .. } => {
        let reach = self.reach(env);
        Bounds::new(-reach, reach)
      }
      Self::Polygon { points } | Self::Polyline { points } => bounds_of(points.iter().map(|p| p.resolve(env))),
      // points only, so dragging a handle doesn't move the center that `Rotate` pivots on
      Self::Bezier { points } => bounds_of(points.iter().map(|p| p.point.resolve(env))),
    }
  }

  /// Outline in local space, before transforms
  fn outline(&self, env: &Env) -> Vec<Vec2> {
    match self {
      Self::Circle { .. } | Self::Ring { .. } | Self::Ellipse { .. } => (0..ROUND_OUTLINE_POINTS)
        .map(|i| Vec2::from_angle(std::f32::consts::TAU * i as f32 / ROUND_OUTLINE_POINTS as f32) * self.reach(env))
        .collect(),
      Self::Rect { .. } => {
        let reach = self.reach(env);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2::new(x, y) * reach).to_vec()
      }
      Self::Pinwheel { radius, blades, blade_fraction } => {
        pinwheel_outline(whole_blades(blades, env), unit_fraction(blade_fraction, env), radius.resolve(env).max(0.0))
      }
      Self::Propeller { radius, blades, blade_width } => {
        propeller_outline(whole_blades(blades, env), Vec2::new(radius.resolve(env), blade_width.resolve(env)).max(Vec2::ZERO))
      }
      // too few points to enclose anything, so it draws nothing
      Self::Polygon { points } if points.len() < 3 => Vec::new(),
      Self::Polygon { points } | Self::Polyline { points } => points.iter().map(|p| p.resolve(env)).collect(),
      Self::Bezier { points } => bezier_line(points, env),
    }
  }

  /// The local-space point `phase` of the way along the outline. Round shapes use the angle directly instead of building
  /// the outline polygon, since a sweep runs this for every sample.
  fn along(&self, phase: f32, env: &Env) -> Vec2 {
    match self {
      Self::Circle { .. } | Self::Ring { .. } | Self::Ellipse { .. } => {
        Vec2::from_angle(std::f32::consts::TAU * phase) * self.reach(env)
      }
      Self::Polyline { .. } | Self::Bezier { .. } => along_line(&self.outline(env), phase),
      _ => along_loop(&self.outline(env), phase),
    }
  }

  /// Half-extents of a shape that isn't made of points. Zero for those that are.
  fn reach(&self, env: &Env) -> Vec2 {
    match self {
      Self::Circle { radius } => Vec2::splat(radius.resolve(env)),
      Self::Ring { inner_radius, thickness } => Vec2::splat(inner_radius.resolve(env).max(0.0) + thickness.resolve(env)),
      Self::Ellipse { rx, ry } => Vec2::new(rx.resolve(env), ry.resolve(env)),
      Self::Rect { width, height } => Vec2::new(width.resolve(env), height.resolve(env)) / 2.0,
      Self::Pinwheel { radius, .. } | Self::Propeller { radius, .. } => Vec2::splat(radius.resolve(env).max(0.0)),
      Self::Polygon { .. } | Self::Polyline { .. } | Self::Bezier { .. } => Vec2::ZERO,
    }
  }

  /// Signed distance to the edge in local space (negative inside), and its gradient
  fn distance(&self, p: Vec2, env: &Env) -> (f32, Vec2) {
    match self {
      Self::Circle { .. } | Self::Ellipse { .. } => ellipse_distance(self.reach(env), p),
      Self::Ring { inner_radius, thickness } => ring_distance(inner_radius.resolve(env), thickness.resolve(env), p),
      Self::Rect { .. } => rect_distance(self.reach(env), p),
      Self::Pinwheel { radius, blades, blade_fraction } => {
        pinwheel_distance(radius.resolve(env), whole_blades(blades, env), unit_fraction(blade_fraction, env), p)
      }
      Self::Propeller { radius, blades, blade_width } => {
        propeller_distance(whole_blades(blades, env), Vec2::new(radius.resolve(env), blade_width.resolve(env)), p)
      }
      Self::Polygon { points } => {
        let points: Vec<Vec2> = points.iter().map(|p| p.resolve(env)).collect();
        polygon_distance(&points, p)
      }
      Self::Polyline { .. } | Self::Bezier { .. } => (f32::INFINITY, Vec2::X),
    }
  }
}

fn bounds_of(mut points: impl Iterator<Item = Vec2>) -> Bounds {
  let Some(first) = points.next() else {
    return Bounds::new(Vec2::ZERO, Vec2::ZERO);
  };
  points.fold(Bounds::new(first, first), |rect, p| Bounds::new(rect.min.min(p), rect.max.max(p)))
}

/// The point `phase` of the way around a closed polygon, by distance
fn along_loop(points: &[Vec2], phase: f32) -> Vec2 {
  let closed: Vec<Vec2> = points.iter().chain(points.first()).copied().collect();
  along_line(&closed, phase)
}

/// The point `phase` of the way along an open polyline, by distance
fn along_line(points: &[Vec2], phase: f32) -> Vec2 {
  let edges = || points.windows(2).map(|edge| (edge[0], edge[1]));
  let mut left = phase * edges().map(|(a, b)| a.distance(b)).sum::<f32>();
  for (a, b) in edges() {
    let length = a.distance(b);
    if left <= length && length > 0.0 {
      return a.lerp(b, left / length);
    }
    left -= length;
  }
  points.last().copied().unwrap_or_default()
}

/// Flattens a bezier into a polyline
fn bezier_line(points: &[BezierPoint], env: &Env) -> Vec<Vec2> {
  let resolved: Vec<(Vec2, Vec2)> = points.iter().map(|p| (p.point.resolve(env), p.handle.resolve(env))).collect();
  let stretches = resolved.windows(2).flat_map(|stretch| {
    let ((from, leaving), (to, arriving)) = (stretch[0], stretch[1]);
    (1..=BEZIER_STEPS).map(move |step| cubic([from, from + leaving, to - arriving, to], step as f32 / BEZIER_STEPS as f32))
  });
  resolved.first().map(|(first, _)| *first).into_iter().chain(stretches).collect()
}

/// The point at `t` on a cubic bezier (de Casteljau)
fn cubic([a, b, c, d]: [Vec2; 4], t: f32) -> Vec2 {
  let (e, f, g) = (a.lerp(b, t), b.lerp(c, t), c.lerp(d, t));
  e.lerp(f, t).lerp(f.lerp(g, t), t)
}

/// Converts a local-space signed distance to scene units, using its gradient. Exact under uniform scale; under
/// non-uniform scale it's only accurate near the edge, which is all coverage needs.
fn scene_distance(distance: f32, gradient: Vec2, to_local: &Affine2) -> f32 {
  let stretch = (to_local.matrix2.transpose() * gradient).length();
  if stretch > f32::EPSILON { distance / stretch } else { distance }
}

/// Anti-aliased coverage: a linear ramp across the sample's radius, so a sample centered on the edge is half covered
fn coverage(distance: f32, radius: f32) -> f32 {
  if radius <= 0.0 {
    return if distance <= 0.0 { 1.0 } else { 0.0 };
  }
  (0.5 - distance / (2.0 * radius)).clamp(0.0, 1.0)
}

/// Approximate signed distance to an ellipse's edge, as a fraction of its radius, with its gradient. Accurate near the
/// edge once divided by the gradient's length (see `scene_distance`).
fn ellipse_distance(reach: Vec2, p: Vec2) -> (f32, Vec2) {
  if reach.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let along = p / reach;
  (along.length() - 1.0, along.normalize_or(Vec2::X) / reach)
}

/// Signed distance to the nearer edge of a ring. With no hole, the outside edge is the only one.
fn ring_distance(hole: f32, thickness: f32, p: Vec2) -> (f32, Vec2) {
  if thickness <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let out = p.normalize_or(Vec2::X);
  let to_outside = p.length() - (hole.max(0.0) + thickness);
  let to_hole = hole - p.length();
  if hole <= 0.0 || to_outside > to_hole { (to_outside, out) } else { (to_hole, -out) }
}

fn whole_blades(blades: &Prop<f32>, env: &Env) -> u32 {
  blades.resolve_truncated(env).floor().clamp(0.0, MAX_BLADES) as u32
}

fn unit_fraction(blade_fraction: &Prop<f32>, env: &Env) -> f32 {
  blade_fraction.resolve(env).clamp(0.0, 1.0)
}

/// Each blade as a wedge: out from the center, along an arc, and back
fn pinwheel_outline(blades: u32, blade_fraction: f32, reach: f32) -> Vec<Vec2> {
  let share = std::f32::consts::TAU / blades.max(1) as f32;
  let width = share * blade_fraction;
  let steps = (ROUND_OUTLINE_POINTS as u32 / (2 * blades.max(1))).max(1);
  (0..blades)
    .flat_map(|blade| {
      let start = blade as f32 * share;
      let end = (0..=steps).map(move |step| Vec2::from_angle(start + width * step as f32 / steps as f32) * reach);
      std::iter::once(Vec2::ZERO).chain(end)
    })
    .collect()
}

/// Signed distance to the nearest blade edge, cut off at the rim. Each side edge is a ray from the center: within 90° of
/// the ray the distance is `r * sin(angle)`, and beyond that the nearest point is the center itself.
fn pinwheel_distance(radius: f32, blades: u32, blade_fraction: f32, p: Vec2) -> (f32, Vec2) {
  if blades == 0 || radius <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let share = std::f32::consts::TAU / blades as f32;
  let width = share * blade_fraction;
  // angle from the center line of the nearest blade
  let around = (p.y.atan2(p.x) - width / 2.0 + share / 2.0).rem_euclid(share) - share / 2.0;
  let right = std::f32::consts::FRAC_PI_2;
  let (sin, cos) = (around.abs() - width / 2.0).clamp(-right, right).sin_cos();
  let out = p.normalize_or(Vec2::X);
  let sides = (p.length() * sin, out * sin + out.perp() * around.signum() * cos);
  // inside both the wedge and the circle, so the farther of the two edges
  let rim = (p.length() - radius, out);
  if rim.0 > sides.0 { rim } else { sides }
}

/// Each blade as a rectangle: out from the center along one side, across the end, and back along the other.
/// `size` is a blade's length and width.
fn propeller_outline(blades: u32, size: Vec2) -> Vec<Vec2> {
  let share = std::f32::consts::TAU / blades.max(1) as f32;
  let side = size.y / 2.0;
  (0..blades)
    .flat_map(|blade| {
      let turn = Vec2::from_angle(blade as f32 * share);
      let corners = [(0.0, -side), (size.x, -side), (size.x, side), (0.0, side)].map(|(x, y)| turn.rotate(Vec2::new(x, y)));
      std::iter::once(Vec2::ZERO).chain(corners)
    })
    .collect()
}

/// Signed distance to the nearest blade. The blades nearest by angle, and the ones either side where they overlap near
/// the center, are each a rectangle from the center out along their angle. `size` is a blade's length and width.
fn propeller_distance(blades: u32, size: Vec2, p: Vec2) -> (f32, Vec2) {
  if blades == 0 || size.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let share = std::f32::consts::TAU / blades as f32;
  let nearest = (p.y.atan2(p.x) / share).round();
  [-1.0, 0.0, 1.0]
    .map(|step| {
      let turn = Vec2::from_angle((nearest + step) * share);
      // into the blade's own space, where it runs along x from the center
      let local = Vec2::new(turn.dot(p), turn.perp_dot(p)) - Vec2::new(size.x / 2.0, 0.0);
      let (distance, gradient) = rect_distance(size / 2.0, local);
      (distance, turn.rotate(gradient))
    })
    .into_iter()
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .unwrap()
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
  use crate::scene::{Key, Keyframes, Params, Transform};
  use crate::scene::value::tests::env;

  fn square() -> ShapeNode {
    rect_polygon(1.0, 1.0)
  }

  fn rect_polygon(width: f32, height: f32) -> ShapeNode {
    ShapeNode::Polygon {
      points: [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
        .into_iter()
        .map(|(x, y)| Vec2::new(x * width, y * height).into())
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
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert_eq!(covered(&circle, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 4.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 6.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn edge_through_a_sample_covers_it_partially() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert!((covered(&circle, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&circle, 5.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn an_ellipse_edge_is_in_scene_units() {
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 2.0.into() });
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_polygon_covers_its_inside() {
    let rect = Shape::new(rect_polygon(10.0, 4.0));
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
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 2.0.into() });
    assert_eq!(covered(&ellipse, 9.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&ellipse, 0.0, 3.0, 0.1), 0.0);
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn an_ellipse_with_no_radius_draws_nothing() {
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 0.0.into() });
    assert_eq!(covered(&ellipse, 0.0, 0.0, 0.5), 0.0);
  }

  fn ring(radius: f32, thickness: f32) -> Shape {
    Shape::new(ShapeNode::Ring { inner_radius: radius.into(), thickness: thickness.into() })
  }

  #[test]
  fn a_circle_reaches_its_radius_every_way() {
    let params = Params::new();
    let circle = ShapeNode::circle(3.0);
    assert_eq!(circle.bounds(&env(0.0, &params)), Bounds::new(Vec2::splat(-3.0), Vec2::splat(3.0)));
  }

  #[test]
  fn a_ring_covers_its_band_and_not_its_hole() {
    let band = ring(4.0, 2.0);
    assert_eq!(covered(&band, 0.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&band, 3.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&band, 0.0, 5.0, 0.1), 1.0);
    assert_eq!(covered(&band, 7.0, 0.0, 0.1), 0.0);
    assert!((covered(&band, 4.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&band, 6.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn a_rings_edges_are_in_scene_units_when_scaled() {
    let band = ring(4.0, 2.0).transform(Transform::Scale(10.0.into()));
    assert!((covered(&band, 40.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&band, 60.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_ring_with_no_hole_is_a_circle() {
    assert_eq!(covered(&ring(0.0, 2.0), 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&ring(0.0, 2.0), 3.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn a_ring_with_no_thickness_draws_nothing() {
    assert_eq!(covered(&ring(4.0, 0.0), 4.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn a_ring_is_bounded_and_gone_around_by_its_outside_edge() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let pose = ring(4.0, 2.0).pose(&frame).unwrap();
    assert_eq!(pose.bounds, Bounds::new(Vec2::splat(-6.0), Vec2::splat(6.0)));
    assert_eq!(pose.hole, 4.0);
    assert!(ring(4.0, 2.0).along(0.0, &frame).abs_diff_eq(Vec2::new(6.0, 0.0), 1e-5));
    assert!(pose.outline.iter().all(|p| (p.length() - 6.0).abs() < 1e-4));
  }

  fn pinwheel(blades: f32) -> Shape {
    pinwheel_with(blades, 0.5)
  }

  fn pinwheel_with(blades: f32, blade_fraction: f32) -> Shape {
    Shape::new(ShapeNode::Pinwheel { radius: 2000.0.into(), blades: blades.into(), blade_fraction: blade_fraction.into() })
  }

  /// How much of a point this many degrees clockwise from the right is covered, far from the middle
  fn covered_at(shape: &Shape, degrees: f32) -> f32 {
    let at = Vec2::from_angle(degrees.to_radians()) * 1000.0;
    covered(shape, at.x, at.y, 0.5)
  }

  #[test]
  fn every_other_stretch_of_a_pinwheel_is_a_blade() {
    let shape = pinwheel(4.0);
    for (degrees, filled) in [(22.5, 1.0), (67.5, 0.0), (112.5, 1.0), (157.5, 0.0), (202.5, 1.0), (247.5, 0.0), (292.5, 1.0), (337.5, 0.0)] {
      assert_eq!(covered_at(&shape, degrees), filled, "at {degrees} degrees");
    }
  }

  #[test]
  fn a_pinwheels_edges_run_straight_out_of_the_middle() {
    let shape = pinwheel(4.0);
    for degrees in [0.0, 45.0, 90.0, 315.0] {
      assert!((covered_at(&shape, degrees) - 0.5).abs() < 0.01, "at {degrees} degrees");
    }
    // half a sample's width to one side of the edge, however far out that is
    assert!((covered(&shape, 500.0, 0.25, 0.5) - 0.75).abs() < 0.01);
    assert!((covered(&shape, 500.0, -0.25, 0.5) - 0.25).abs() < 0.01);
  }

  #[test]
  fn a_pinwheel_of_one_blade_is_half_of_everything() {
    assert_eq!(covered_at(&pinwheel(1.0), 90.0), 1.0);
    assert_eq!(covered_at(&pinwheel(1.0), 270.0), 0.0);
  }

  #[test]
  fn a_pinwheel_with_no_blades_draws_nothing() {
    let params = Params::new();
    assert_eq!(covered_at(&pinwheel(0.0), 22.5), 0.0);
    assert_eq!(pinwheel(0.0).pose(&env(0.0, &params)).unwrap().outline, vec![]);
  }

  #[test]
  fn a_fraction_of_a_blade_is_dropped() {
    assert_eq!(covered_at(&pinwheel(1.9), 90.0), 1.0);
    assert_eq!(covered_at(&pinwheel(1.9), 270.0), 0.0);
  }

  #[test]
  fn a_pinwheels_blade_fraction_is_its_share_of_each_slice() {
    let shape = pinwheel_with(4.0, 0.25);
    for (degrees, filled) in [(5.0, 1.0), (17.0, 1.0), (30.0, 0.0), (85.0, 0.0), (95.0, 1.0), (120.0, 0.0)] {
      assert_eq!(covered_at(&shape, degrees), filled, "at {degrees} degrees");
    }
    assert!((covered_at(&shape, 22.5) - 0.5).abs() < 0.01);
  }

  #[test]
  fn past_a_right_angle_from_a_blades_edge_the_middle_is_nearest() {
    let behind = |blade_fraction: f32| Vec2::from_angle(std::f32::consts::PI * (blade_fraction + 1.0)) * 10.0;
    let middle = |blade_fraction: f32| Vec2::from_angle(std::f32::consts::PI * blade_fraction) * 10.0;
    assert!((pinwheel_distance(100.0, 1, 0.1, behind(0.1)).0 - 10.0).abs() < 1e-4);
    assert!((pinwheel_distance(100.0, 1, 0.9, middle(0.9)).0 + 10.0).abs() < 1e-4);
    assert_eq!(covered(&pinwheel_with(1.0, 0.9), 10.0, -0.1, 0.05), 0.0);
    assert_eq!(covered(&pinwheel_with(1.0, 0.9), -10.0, 0.0, 0.05), 1.0);
  }

  #[test]
  fn a_pinwheel_turns_with_a_rotate() {
    let shape = pinwheel(4.0).transform(Transform::Rotate(45.0.into()));
    assert_eq!(covered_at(&shape, 22.5), 0.0);
    assert_eq!(covered_at(&shape, 67.5), 1.0);
  }

  #[test]
  fn a_pinwheel_can_be_moved_to_a_new_middle() {
    let shape = pinwheel(4.0).transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
    assert_eq!(covered(&shape, 60.0, 29.0, 0.5), 1.0);
    assert_eq!(covered(&shape, 54.0, 35.0, 0.5), 0.0);
  }

  #[test]
  fn a_pinwheel_is_as_big_as_its_radius() {
    let params = Params::new();
    let shape = Shape::new(ShapeNode::Pinwheel { radius: 20.0.into(), blades: 4.0.into(), blade_fraction: 0.5.into() });
    let pose = shape.pose(&env(0.0, &params)).unwrap();
    assert_eq!(pose.bounds, Bounds::new(Vec2::splat(-20.0), Vec2::splat(20.0)));
    assert_eq!(pose.outline.iter().filter(|point| **point == Vec2::ZERO).count(), 4);
    assert!(pose.outline.iter().all(|point| *point == Vec2::ZERO || (point.length() - 20.0).abs() < 1e-3));
    let out = |length: f32| Vec2::from_angle(22.5f32.to_radians()) * length;
    assert_eq!(covered(&shape, out(19.0).x, out(19.0).y, 0.5), 1.0);
    assert_eq!(covered(&shape, out(21.0).x, out(21.0).y, 0.5), 0.0);
    assert!((covered(&shape, out(20.0).x, out(20.0).y, 0.5) - 0.5).abs() < 1e-3);
  }

  #[test]
  fn a_pinwheel_with_no_radius_draws_nothing() {
    let shape = Shape::new(ShapeNode::Pinwheel { radius: 0.0.into(), blades: 4.0.into(), blade_fraction: 0.5.into() });
    assert_eq!(covered(&shape, 0.0, 0.0, 0.5), 0.0);
  }

  fn propeller(blades: f32, length: f32, width: f32) -> Shape {
    Shape::new(ShapeNode::Propeller { radius: length.into(), blades: blades.into(), blade_width: width.into() })
  }

  #[test]
  fn a_propellers_blades_are_rectangles_out_from_the_middle() {
    let shape = propeller(4.0, 20.0, 4.0);
    // along each blade, and to either side of one
    for (x, y) in [(10.0, 0.0), (0.0, 10.0), (-10.0, 0.0), (0.0, -10.0), (10.0, 1.5), (19.0, -1.5)] {
      assert_eq!(covered(&shape, x, y, 0.5), 1.0, "at ({x}, {y})");
    }
    // past a blade's side, past its end, and between blades
    for (x, y) in [(10.0, 3.0), (21.0, 0.0), (10.0, 10.0)] {
      assert_eq!(covered(&shape, x, y, 0.5), 0.0, "at ({x}, {y})");
    }
    assert!((covered(&shape, 10.0, 2.0, 0.5) - 0.5).abs() < 1e-3);
    assert!((covered(&shape, 20.0, 0.0, 0.5) - 0.5).abs() < 1e-3);
  }

  #[test]
  fn a_propellers_blade_keeps_its_width_however_far_out() {
    let shape = propeller(3.0, 100.0, 2.0);
    let along = Vec2::from_angle(120f32.to_radians());
    for length in [5.0, 50.0, 95.0] {
      let side = along * length + along.perp() * 0.8;
      assert_eq!(covered(&shape, side.x, side.y, 0.1), 1.0, "at {length}");
      let past = along * length + along.perp() * 1.2;
      assert_eq!(covered(&shape, past.x, past.y, 0.1), 0.0, "at {length}");
    }
  }

  #[test]
  fn a_propeller_is_as_big_as_its_radius() {
    let params = Params::new();
    let pose = propeller(2.0, 20.0, 4.0).pose(&env(0.0, &params)).unwrap();
    assert_eq!(pose.bounds, Bounds::new(Vec2::splat(-20.0), Vec2::splat(20.0)));
    assert_eq!(pose.outline.len(), 10);
    assert!(pose.outline.iter().all(|point| point.x.abs() <= 20.0 + 1e-4 && point.y.abs() <= 2.0 + 1e-4));
  }

  #[test]
  fn a_propeller_with_no_blades_or_width_draws_nothing() {
    let params = Params::new();
    assert_eq!(covered(&propeller(0.0, 20.0, 4.0), 10.0, 0.0, 0.5), 0.0);
    assert_eq!(covered(&propeller(4.0, 20.0, 0.0), 10.0, 0.0, 0.5), 0.0);
    assert_eq!(propeller(0.0, 20.0, 4.0).pose(&env(0.0, &params)).unwrap().outline, vec![]);
  }

  #[test]
  fn a_pinwheel_has_only_so_many_blades() {
    let params = Params::new();
    assert_eq!(pinwheel(1e9).pose(&env(0.0, &params)).unwrap().outline.len(), MAX_BLADES as usize * 3);
  }

  fn polyline(points: &[(f32, f32)]) -> ShapeNode {
    ShapeNode::Polyline { points: points.iter().map(|(x, y)| Vec2::new(*x, *y).into()).collect() }
  }

  fn bezier(points: &[((f32, f32), (f32, f32))]) -> ShapeNode {
    let points = points.iter().map(|(point, handle)| BezierPoint { point: Vec2::from(*point).into(), handle: Vec2::from(*handle).into() });
    ShapeNode::Bezier { points: points.collect() }
  }

  #[test]
  fn a_polyline_goes_from_its_first_point_to_its_last_at_an_even_speed() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0)]));
    for (phase, x, y) in [(0.0, 0.0, 0.0), (0.5, 20.0, 0.0), (0.75, 30.0, 0.0), (1.0, 30.0, 10.0)] {
      assert!(path.along(phase, &frame).abs_diff_eq(Vec2::new(x, y), 1e-4), "at {phase}");
    }
  }

  #[test]
  fn a_polyline_of_one_point_stays_on_it_and_one_of_none_at_the_origin() {
    let params = Params::new();
    let frame = env(0.0, &params);
    assert_eq!(Shape::new(polyline(&[(3.0, 4.0)])).along(0.5, &frame), Vec2::new(3.0, 4.0));
    assert_eq!(Shape::new(polyline(&[])).along(0.5, &frame), Vec2::ZERO);
  }

  #[test]
  fn a_line_has_no_inside_to_draw() {
    let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    assert_eq!(covered(&Shape::new(polyline(&square)), 5.0, 5.0, 0.5), 0.0);
    assert_eq!(covered(&Shape::new(polyline(&square)), 5.0, 0.0, 0.5), 0.0);
    let curve = bezier(&[((0.0, 0.0), (0.0, 10.0)), ((10.0, 0.0), (0.0, -10.0))]);
    assert_eq!(covered(&Shape::new(curve), 5.0, 5.0, 0.5), 0.0);
  }

  #[test]
  fn a_line_does_not_close() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let pose = Shape::new(polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0)])).pose(&frame).unwrap();
    assert_eq!(pose.outline.len(), 3);
    assert_eq!(pose.points, pose.outline);
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(30.0, 10.0)));
    assert_eq!(pose.handles, vec![]);
  }

  #[test]
  fn a_bezier_with_no_handles_is_a_polyline() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(bezier(&[((0.0, 0.0), (0.0, 0.0)), ((30.0, 0.0), (0.0, 0.0)), ((30.0, 10.0), (0.0, 0.0))]));
    assert!(path.along(0.5, &frame).abs_diff_eq(Vec2::new(20.0, 0.0), 1e-3));
  }

  #[test]
  fn a_bezier_goes_through_its_points_and_leans_toward_its_handles() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(bezier(&[((0.0, 0.0), (0.0, 12.0)), ((20.0, 0.0), (0.0, -12.0))]));
    assert_eq!(path.along(0.0, &frame), Vec2::ZERO);
    assert!(path.along(1.0, &frame).abs_diff_eq(Vec2::new(20.0, 0.0), 1e-4));
    // the same arch both ways, and 3/4 of the way out to the handles at its top
    assert!(path.along(0.5, &frame).abs_diff_eq(Vec2::new(10.0, 9.0), 1e-2));
  }

  #[test]
  fn a_handle_stays_with_its_point_and_arrives_from_the_opposite_way() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = ShapeNode::Bezier {
      points: vec![
        BezierPoint { point: Vec2::ZERO.into(), handle: Vec2::ZERO.into() },
        BezierPoint { point: Vec2::new(10.0, 0.0).into(), handle: Vec2::new(0.0, 5.0).into() },
        BezierPoint { point: Vec2::new(20.0, 0.0).into(), handle: Vec2::ZERO.into() },
      ],
    };
    let line = path.outline(&frame);
    let middle = line[BEZIER_STEPS];
    assert_eq!(middle, Vec2::new(10.0, 0.0));
    let (before, after) = (line[BEZIER_STEPS - 1] - middle, line[BEZIER_STEPS + 1] - middle);
    assert!(before.normalize().dot(after.normalize()) < -0.999, "turns a corner at the point");
    assert!(after.y > 0.0 && before.y < 0.0);
  }

  #[test]
  fn a_beziers_middle_is_that_of_its_points() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let pose = Shape::new(bezier(&[((0.0, 0.0), (0.0, 100.0)), ((20.0, 10.0), (0.0, -100.0))])).pose(&frame).unwrap();
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(20.0, 10.0)));
    assert_eq!(pose.points, vec![Vec2::ZERO, Vec2::new(20.0, 10.0)]);
    assert_eq!(pose.handles, vec![Vec2::new(0.0, 100.0), Vec2::new(0.0, -100.0)]);
    assert_eq!(pose.outline.len(), 1 + BEZIER_STEPS);
  }

  #[test]
  fn a_bezier_is_animated_by_a_point_or_a_handle() {
    let moves = || Prop::Keyframes(Keyframes { keys: vec![Key::new(0.0, Vec2::ZERO, Curve::Linear)], cycle: Cycle::Once });
    let by_point = BezierPoint { point: moves(), handle: Vec2::ZERO.into() };
    let by_handle = BezierPoint { point: Vec2::ZERO.into(), handle: moves() };
    assert!(ShapeNode::Bezier { points: vec![by_point] }.is_animated());
    assert!(ShapeNode::Bezier { points: vec![by_handle] }.is_animated());
    assert!(!bezier(&[((0.0, 0.0), (1.0, 1.0))]).is_animated());
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
    let rect = Shape::new(ShapeNode::Rect { width: 1.0.into(), height: 0.2.into() }).transform(Transform::Scale(20.0.into()));
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
    let grows = Prop::Keyframes(Keyframes {
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
    let flat = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Scale(0.0.into()));
    assert_eq!(flat.pose(&env(0.0, &params)), None);
  }

  #[test]
  fn zero_radius_is_a_hard_edge() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert_eq!(covered(&circle, 4.99, 0.0, 0.0), 1.0);
    assert_eq!(covered(&circle, 5.01, 0.0, 0.0), 0.0);
  }
}
