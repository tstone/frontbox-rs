use glam::{Affine2, Vec2};

/// Line segments per bulged side
const BULGE_STEPS: usize = 16;

/// Line segments in a full turn of a rounded corner's arc
const ARC_STEPS: f32 = 64.0;

/// A shape's outline as points, with which are corners and which edges are straight lines, for the transforms that
/// reshape it: a `Shift`, `Bulge`, or `Round`
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Contour {
  points: Vec<ContourPoint>,
  closed: bool,
}

impl Contour {
  /// Straight edges between corners, as a polygon or a polyline has. A line's ends aren't corners.
  pub(crate) fn straight(points: Vec<Vec2>, closed: bool) -> Self {
    let last = points.len().saturating_sub(1);
    let point = |(i, at): (usize, Vec2)| ContourPoint {
      at,
      corner: closed || (i > 0 && i < last),
      straight: true,
    };
    Self {
      points: points.into_iter().enumerate().map(point).collect(),
      closed,
    }
  }

  /// A smooth curve broken into short edges, as a circle or a bezier is, with no corners or straight edges
  pub(crate) fn curved(points: Vec<Vec2>, closed: bool) -> Self {
    Self {
      points: points
        .into_iter()
        .map(|at| ContourPoint {
          at,
          corner: false,
          straight: false,
        })
        .collect(),
      closed,
    }
  }

  pub(crate) fn points(&self) -> Vec<Vec2> {
    self.points.iter().map(|point| point.at).collect()
  }

  pub(crate) fn transformed(&self, affine: Affine2) -> Self {
    let points = self
      .points
      .iter()
      .map(|point| ContourPoint {
        at: affine.transform_point2(point.at),
        ..*point
      })
      .collect();
    Self {
      points,
      closed: self.closed,
    }
  }

  /// Each point moved by `moved`, with a point added where an edge's `along` changes sign. A straight edge stays
  /// straight either side of it, and turns a corner there.
  pub(crate) fn split_and_moved(
    &self,
    along: impl Fn(Vec2) -> f32,
    moved: impl Fn(Vec2) -> Vec2,
  ) -> Self {
    let edges = self.edges();
    let mut points = Vec::with_capacity(self.points.len() * 2);
    for i in 0..self.points.len() {
      let (from, to) = (self.points[i], self.points[(i + 1) % self.points.len()]);
      points.push(ContourPoint {
        at: moved(from.at),
        ..from
      });
      let (from_t, to_t) = (along(from.at), along(to.at));
      if i < edges && from_t * to_t < 0.0 {
        let split = from.at.lerp(to.at, from_t / (from_t - to_t));
        points.push(ContourPoint {
          at: moved(split),
          corner: from.straight,
          straight: from.straight,
        });
      }
    }
    Self {
      points,
      closed: self.closed,
    }
  }

  /// Each straight edge curved out from the inside by `amount` of its length at its middle, or in when negative. A
  /// line has no inside, so it's taken as the side the line bends around.
  pub(crate) fn bulged(&self, amount: f32) -> Self {
    if amount == 0.0 {
      return self.clone();
    }
    let edges = self.edges();
    // which way each edge's `perp` points, inside or out
    let inward = if self.winding() < 0.0 { -1.0 } else { 1.0 };
    let mut points = Vec::with_capacity(self.points.len() * BULGE_STEPS);
    for i in 0..self.points.len() {
      let from = self.points[i];
      if i >= edges || !from.straight {
        points.push(from);
        continue;
      }
      let to = self.points[(i + 1) % self.points.len()].at;
      // a quadratic bezier reaches halfway to its control point at its middle
      let control = (from.at + to) / 2.0 - (to - from.at).perp() * inward * amount * 2.0;
      points.push(ContourPoint {
        straight: false,
        ..from
      });
      points.extend((1..BULGE_STEPS).map(|step| {
        let t = step as f32 / BULGE_STEPS as f32;
        ContourPoint {
          at: from.at.lerp(control, t).lerp(control.lerp(to, t), t),
          corner: false,
          straight: false,
        }
      }));
    }
    Self {
      points,
      closed: self.closed,
    }
  }

  /// Each corner cut back and joined by an arc `radius` across, which meets the edges either side without a bend. Where
  /// an edge is too short for that, its corners share it and their arcs are tighter. An arc meeting a curve is an
  /// approximation of a circle's.
  pub(crate) fn rounded(&self, radius: f32) -> Self {
    let last = self.points.len().saturating_sub(1);
    let corners: Vec<usize> = (0..self.points.len())
      .filter(|i| self.points[*i].corner && (self.closed || (*i > 0 && *i < last)))
      .collect();
    if radius <= 0.0 || corners.is_empty() {
      return self.clone();
    }
    // each stretch from one corner, or a line's start, to the next corner, or the line's end
    let stops: Vec<usize> = match self.closed {
      true => corners
        .iter()
        .copied()
        .chain([corners[0] + self.points.len()])
        .collect(),
      false => [0]
        .into_iter()
        .chain(corners.iter().copied())
        .chain([last])
        .collect(),
    };
    let sides: Vec<Side> = stops
      .windows(2)
      .map(|ends| {
        Side::new(
          (ends[0]..=ends[1])
            .map(|i| self.points[i % self.points.len()])
            .collect(),
        )
      })
      .collect();
    // how far back each corner is cut along the sides either side of it; a side is cut at both ends when both are
    // corners, so each gets half
    let share = |side: &Side, other_end_is_corner: bool| {
      if other_end_is_corner {
        side.length / 2.0
      } else {
        side.length
      }
    };
    let joins = sides.len() - usize::from(!self.closed);
    let cuts: Vec<f32> = (0..joins)
      .map(|join| {
        let (before, after) = (&sides[join], &sides[(join + 1) % sides.len()]);
        let turn = before
          .leaving()
          .zip(after.entering())
          .map_or(0.0, |(arriving, leaving)| arriving.angle_to(leaving).abs());
        let wanted = if turn > f32::EPSILON {
          radius * (turn / 2.0).tan()
        } else {
          0.0
        };
        let first = !self.closed && join == 0;
        let last = !self.closed && join == joins - 1;
        wanted.min(share(before, !first)).min(share(after, !last))
      })
      .collect();
    let mut points = Vec::with_capacity(self.points.len() + corners.len() * ARC_STEPS as usize);
    for (index, side) in sides.iter().enumerate() {
      // in a closed outline every side starts at a corner, and in a line every side but the first does
      let corner = if self.closed {
        Some((index + joins - 1) % joins)
      } else {
        index.checked_sub(1)
      };
      let start = match corner {
        Some(join) => {
          let cut = cuts[join];
          points.extend(sides[(index + sides.len() - 1) % sides.len()].arc_to(side, cut));
          cut
        }
        None => {
          points.push(side.points[0]);
          0.0
        }
      };
      let end = if self.closed || index < sides.len() - 1 {
        side.length - cuts[index % joins]
      } else {
        side.length
      };
      points.extend(side.between(start, end));
      if !self.closed && index == sides.len() - 1 {
        points.push(*side.points.last().unwrap());
      }
    }
    Self {
      points,
      closed: self.closed,
    }
  }

  /// How many of the points start an edge
  fn edges(&self) -> usize {
    if self.closed {
      self.points.len()
    } else {
      self.points.len().saturating_sub(1)
    }
  }

  /// Twice the area enclosed, positive when each edge's `perp` points inside and negative when it points out. A line
  /// is taken as closed from its end back to its start.
  fn winding(&self) -> f32 {
    let n = self.points.len();
    (0..n)
      .map(|i| self.points[i].at.perp_dot(self.points[(i + 1) % n].at))
      .sum()
  }
}

/// A point of a [`Contour`]
#[derive(Debug, Clone, Copy, PartialEq)]
struct ContourPoint {
  at: Vec2,
  /// Whether the outline turns a corner here, rather than going on along a curve
  corner: bool,
  /// Whether the edge from here to the next point is a straight line, rather than part of a curve
  straight: bool,
}

/// The points of a contour from one corner, or a line's end, to the next, with both ends
struct Side {
  points: Vec<ContourPoint>,
  /// How far along the side each point is
  distances: Vec<f32>,
  length: f32,
}

impl Side {
  fn new(points: Vec<ContourPoint>) -> Self {
    let distances: Vec<f32> = points
      .iter()
      .scan((0.0, points[0].at), |(total, previous), point| {
        *total += previous.distance(point.at);
        *previous = point.at;
        Some(*total)
      })
      .collect();
    Self {
      length: *distances.last().unwrap(),
      points,
      distances,
    }
  }

  /// Which way it goes from its start, if it goes anywhere
  fn entering(&self) -> Option<Vec2> {
    self
      .points
      .iter()
      .find_map(|point| (point.at - self.points[0].at).try_normalize())
  }

  /// Which way it goes into its end, if it comes from anywhere
  fn leaving(&self) -> Option<Vec2> {
    let end = self.points.last()?.at;
    self
      .points
      .iter()
      .rev()
      .find_map(|point| (end - point.at).try_normalize())
  }

  /// The point `distance` along it, which way it goes there, and whether the edge it's on is straight
  fn at(&self, distance: f32) -> (Vec2, Vec2, bool) {
    let edge = self
      .distances
      .iter()
      .position(|d| *d >= distance)
      .unwrap_or(self.distances.len() - 1)
      .max(1)
      - 1;
    let (from, to) = (self.points[edge], self.points[edge + 1]);
    let length = self.distances[edge + 1] - self.distances[edge];
    let t = if length > 0.0 {
      (distance - self.distances[edge]) / length
    } else {
      0.0
    };
    let direction = (to.at - from.at)
      .try_normalize()
      .or_else(|| self.leaving())
      .unwrap_or(Vec2::X);
    (from.at.lerp(to.at, t), direction, from.straight)
  }

  /// The points strictly between `start` and `end` along it
  fn between(&self, start: f32, end: f32) -> impl Iterator<Item = ContourPoint> + '_ {
    self
      .points
      .iter()
      .zip(&self.distances)
      .filter(move |(_, d)| **d > start && **d < end)
      .map(|(point, _)| *point)
  }

  /// The arc around the corner where this side ends and `next` starts, from `cut` back along this one to `cut` along
  /// `next`, both ends included. Just the corner, without a cut.
  fn arc_to(&self, next: &Side, cut: f32) -> Vec<ContourPoint> {
    let corner = *self.points.last().unwrap();
    if cut <= 0.0 {
      return vec![corner];
    }
    let (from, arriving, _) = self.at(self.length - cut);
    let (to, leaving, straight) = next.at(cut);
    // where the edges either side would meet, which is the corner itself when they're straight
    let across = arriving.perp_dot(leaving);
    let ahead = if across.abs() > f32::EPSILON {
      (to - from).perp_dot(leaving) / across
    } else {
      0.0
    };
    let control = if ahead > 0.0 {
      from + arriving * ahead
    } else {
      corner.at
    };
    let turn = arriving.angle_to(leaving).abs();
    // a rational quadratic bezier with this weight is an arc of a circle, when it's cut back the same both ways
    let weight = (turn / 2.0).cos();
    let steps = (turn / std::f32::consts::TAU * ARC_STEPS).ceil().max(1.0) as usize;
    let point = |step: usize| {
      let t = step as f32 / steps as f32;
      let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * weight * t * (1.0 - t), t * t);
      let at = (from * a + control * b + to * c) / (a + b + c);
      ContourPoint {
        at,
        corner: false,
        straight: step == steps && straight,
      }
    };
    (0..=steps).map(point).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn square(size: f32) -> Contour {
    Contour::straight(
      vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(size, 0.0),
        Vec2::new(size, size),
        Vec2::new(0.0, size),
      ],
      true,
    )
  }

  #[test]
  fn a_bulge_curves_each_side_out_by_its_share_of_the_side() {
    for contour in [
      square(10.0),
      Contour::straight(square(10.0).points().into_iter().rev().collect(), true),
    ] {
      let points = contour.bulged(0.25).points();
      assert!(
        points
          .iter()
          .any(|p| p.abs_diff_eq(Vec2::new(5.0, -2.5), 1e-4)),
        "{points:?}"
      );
      assert!(points.iter().all(|p| p.y >= -2.5 - 1e-4));
    }
    let inward = square(10.0).bulged(-0.25).points();
    assert!(
      inward
        .iter()
        .any(|p| p.abs_diff_eq(Vec2::new(5.0, 2.5), 1e-4))
    );
  }

  #[test]
  fn a_bulge_leaves_curves_alone() {
    let circle = Contour::curved(vec![Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y], true);
    assert_eq!(circle.bulged(0.5), circle);
  }

  #[test]
  fn a_round_corner_is_an_arc_that_meets_both_sides() {
    let rounded = square(10.0).rounded(2.0).points();
    // the top left corner's arc goes from 2 down its left side to 2 along its top, around (2, 2)
    assert!(
      rounded
        .iter()
        .any(|p| p.abs_diff_eq(Vec2::new(0.0, 2.0), 1e-4))
    );
    assert!(
      rounded
        .iter()
        .any(|p| p.abs_diff_eq(Vec2::new(2.0, 0.0), 1e-4))
    );
    let arc: Vec<&Vec2> = rounded.iter().filter(|p| p.x < 2.0 && p.y < 2.0).collect();
    assert!(arc.len() > 2);
    assert!(
      arc
        .iter()
        .all(|p| (p.distance(Vec2::new(2.0, 2.0)) - 2.0).abs() < 1e-3),
      "{arc:?}"
    );
    assert!(!rounded.iter().any(|p| p.abs_diff_eq(Vec2::ZERO, 1e-4)));
  }

  #[test]
  fn corners_share_a_side_too_short_for_both_arcs() {
    let rounded = square(10.0).rounded(100.0).points();
    // each cut back half the side, so it's a circle
    assert!(
      rounded
        .iter()
        .all(|p| (p.distance(Vec2::splat(5.0)) - 5.0).abs() < 1e-3),
      "{rounded:?}"
    );
  }

  #[test]
  fn a_round_leaves_a_lines_ends_and_rounds_its_bends() {
    let line = Contour::straight(
      vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(10.0, 10.0),
      ],
      false,
    );
    let rounded = line.rounded(2.0).points();
    assert_eq!(rounded.first(), Some(&Vec2::ZERO));
    assert_eq!(rounded.last(), Some(&Vec2::new(10.0, 10.0)));
    assert!(
      rounded
        .iter()
        .any(|p| p.abs_diff_eq(Vec2::new(8.0, 0.0), 1e-4))
    );
    assert!(
      !rounded
        .iter()
        .any(|p| p.abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4))
    );
  }

  #[test]
  fn a_round_after_a_bulge_rounds_where_the_curves_meet() {
    let rounded = square(10.0).bulged(0.1).rounded(1.0);
    let points = rounded.points();
    assert!(!points.iter().any(|p| p.abs_diff_eq(Vec2::ZERO, 1e-3)));
    // still goes around the same way
    assert!(rounded.winding() > 0.0);
  }
}
