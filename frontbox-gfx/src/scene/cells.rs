use std::f32::consts::TAU;

use glam::{IVec2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::noise::hash;

/// The look of a [`FillNode::Cells`](crate::scene::FillNode::Cells): what's drawn of the cells around scattered points
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellStyle {
  /// 1 where cells meet, falling to 0 at their middles, for cracks, scales, and crazed glass. Two stops close together
  /// near 1 draw it as a web of thin lines.
  #[default]
  Edges,
  /// A flat random shade from 0 to 1 per cell, for mosaic and stained glass
  Value,
}

impl CellStyle {
  pub(crate) fn is_edges(&self) -> bool {
    *self == Self::Edges
  }

  /// The cells at `p`, 0 to 1, where a cell is 1 across on average. Each cell is the area nearest one point, which is
  /// pushed off its grid square's middle by up to `jitter` (0 is a grid of squares, 1 is fully random) and circles
  /// around it with `evolution`, coming back at every whole number.
  pub(crate) fn sample(self, p: Vec2, jitter: f32, evolution: f32, seed: u32) -> f32 {
    let square = p.floor();
    let (square, f) = (square.as_ivec2(), p - square);
    let jitter = jitter.clamp(0.0, 1.0);
    // from p to the point of the square `offset` from p's
    let to_point =
      |offset: IVec2| offset.as_vec2() + point(square + offset, jitter, evolution, seed) - f;

    // points stay within half a square of their square's middle, so the nearest is in one of the 9 around p
    let (mut nearest, mut to_nearest, mut distance) = (IVec2::ZERO, Vec2::ZERO, f32::MAX);
    for offset in neighbors(IVec2::ZERO, 1) {
      let to = to_point(offset);
      if to.length_squared() < distance {
        (nearest, to_nearest, distance) = (offset, to, to.length_squared());
      }
    }

    match self {
      Self::Value => unit(hash(square.x + nearest.x, square.y + nearest.y, 1, seed)),
      Self::Edges => {
        // the distance to the nearest edge: to the line halfway between the nearest point and each other one, along the
        // way between them (Inigo Quilez's exact border distance)
        let edge = neighbors(nearest, 2)
          .filter(|&offset| offset != nearest)
          .map(|offset| {
            let to = to_point(offset);
            (to_nearest + to).dot((to - to_nearest).normalize_or_zero()) * 0.5
          })
          .fold(f32::MAX, f32::min);
        (1.0 - edge * 2.0).clamp(0.0, 1.0)
      }
    }
  }
}

/// The squares within `reach` of `middle` along each axis
fn neighbors(middle: IVec2, reach: i32) -> impl Iterator<Item = IVec2> {
  (-reach..=reach).flat_map(move |y| (-reach..=reach).map(move |x| middle + IVec2::new(x, y)))
}

/// Where the point of grid square `square` is, from its corner: its middle, pushed out up to `jitter` of half a square
/// and turned around it by `evolution`, one way or the other
fn point(square: IVec2, jitter: f32, evolution: f32, seed: u32) -> Vec2 {
  let h = hash(square.x, square.y, 0, seed);
  // the square root spreads points evenly over the disc, rather than bunched in its middle
  let radius = unit(h & 0xffff).sqrt() * jitter * 0.5;
  let turn = unit((h >> 16) & 0x7fff) * 2.0;
  let way = if h >> 31 == 0 { 1.0 } else { -1.0 };
  Vec2::splat(0.5) + Vec2::from_angle((turn + evolution * way) * TAU) * radius
}

/// The low 16 bits of `bits` from 0 to 1
fn unit(bits: u32) -> f32 {
  (bits & 0xffff) as f32 / 65_535.0
}

#[cfg(test)]
mod tests {
  use super::*;

  fn grid() -> impl Iterator<Item = Vec2> {
    (0..40).flat_map(|x| (0..40).map(move |y| Vec2::new(x as f32, y as f32) * 0.173 - 3.0))
  }

  #[test]
  fn stays_between_zero_and_one() {
    for style in [CellStyle::Edges, CellStyle::Value] {
      for p in grid() {
        let n = style.sample(p, 1.0, 0.3, 1);
        assert!((0.0..=1.0).contains(&n), "{style:?} gave {n} at {p}");
      }
    }
  }

  #[test]
  fn without_jitter_edges_are_the_grid() {
    let edges = |x: f32, y: f32| CellStyle::Edges.sample(Vec2::new(x, y), 0.0, 0.0, 1);
    assert!((edges(2.0, 1.5) - 1.0).abs() < 1e-4);
    assert!((edges(1.5, -3.0) - 1.0).abs() < 1e-4);
    assert!(edges(1.5, 1.5).abs() < 1e-4);
    assert!((edges(1.25, 1.5) - 0.5).abs() < 1e-4);
  }

  #[test]
  fn value_is_flat_within_a_cell() {
    let value = |x: f32, y: f32| CellStyle::Value.sample(Vec2::new(x, y), 0.0, 0.0, 1);
    assert_eq!(value(0.1, 0.1), value(0.9, 0.8));
    let shades: Vec<f32> = (0..20).map(|x| value(x as f32 + 0.5, 0.5)).collect();
    assert!(
      shades.windows(2).any(|pair| pair[0] != pair[1]),
      "{shades:?}"
    );
  }

  #[test]
  fn edges_are_continuous() {
    for p in grid().take(400) {
      let at = |p: Vec2| CellStyle::Edges.sample(p, 1.0, 0.2, 1);
      let step = (at(p) - at(p + Vec2::splat(0.001))).abs();
      assert!(step < 0.05, "jumped {step} at {p}");
    }
  }

  #[test]
  fn edges_reach_both_ends() {
    let (low, high) = grid().fold((1.0f32, 0.0f32), |(low, high), p| {
      let n = CellStyle::Edges.sample(p, 1.0, 0.0, 1);
      (low.min(n), high.max(n))
    });
    assert!(low < 0.3 && high > 0.95, "only spans {low} to {high}");
  }

  #[test]
  fn evolution_moves_the_cells_and_loops_at_every_whole_number() {
    for style in [CellStyle::Edges, CellStyle::Value] {
      let at = |p: Vec2, evolution: f32| style.sample(p, 1.0, evolution, 1);
      assert!(grid().any(|p| (at(p, 0.0) - at(p, 0.3)).abs() > 1e-3));
      for p in grid().take(200) {
        assert!((at(p, 0.25) - at(p, 1.25)).abs() < 1e-3, "{style:?} at {p}");
      }
    }
  }

  #[test]
  fn the_same_for_a_seed_and_different_between_seeds() {
    let at = |seed: u32| -> Vec<f32> {
      grid()
        .take(100)
        .map(|p| CellStyle::Edges.sample(p, 1.0, 0.0, seed))
        .collect()
    };
    assert_eq!(at(1), at(1));
    assert_ne!(at(1), at(2));
  }
}
