use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Lattice cells of change on the evolution axis in one period of evolution, on the first octave. Each octave after
/// doubles it with the frequency, so every octave comes back around at the same time.
const EVOLUTION_CELLS: i32 = 2;
/// Most octaves a noise sums. Past this, each is far smaller than a pixel or a light even on a large cell.
pub const MAX_OCTAVES: u32 = 8;

/// The look of a [`FillNode::Noise`](crate::scene::FillNode::Noise): which shape each octave's noise is folded into
/// before they're summed
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseStyle {
  /// Soft rolling blobs, for clouds, smoke, and fog
  #[default]
  Smooth,
  /// Folded where it crosses the middle, so it runs low with soft bright billows, for fire and churning smoke
  Turbulent,
  /// The inverse of `Turbulent`, sharpened: thin bright veins where the noise crosses the middle, for lightning, arcs,
  /// and marble
  Ridged,
}

impl NoiseStyle {
  pub(crate) fn is_smooth(&self) -> bool {
    *self == Self::Smooth
  }

  /// The noise at `p`, 0 to 1, where a cell is 1 across. `evolution` changes it in place and comes back around at
  /// every whole number, so animating it from 0 to a whole number loops.
  pub(crate) fn sample(self, p: Vec2, evolution: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amplitude, mut total, mut squares) = (0.0, 1.0, 0.0, 0.0);
    for octave in 0..octaves.clamp(1, MAX_OCTAVES) {
      let frequency = 1 << octave;
      // so the octaves' lattices don't share their zeros at the origin, and none is sliced along a lattice plane, where
      // gradient noise is at its most blocky. Shifting along z keeps it looping, as z repeats by whole cells.
      let offset = Vec3::splat((octave + 1) as f32 * 0.618_034).fract();
      let period = EVOLUTION_CELLS * frequency;
      let at = (p * frequency as f32).extend(evolution * period as f32) + offset;
      let n = gradient_noise(at, period, seed.wrapping_add(octave));
      let folded = match self {
        Self::Smooth => n,
        Self::Turbulent => n.abs(),
        Self::Ridged => (1.0 - n.abs()).powi(2),
      };
      sum += folded * amplitude;
      total += amplitude;
      squares += amplitude * amplitude;
      amplitude *= 0.5;
    }
    // octaves partly cancel, so their average strays less far from the middle than one does. Spreading it back out by
    // how much less keeps the look's contrast the same at any number of octaves.
    let (middle, low, high) = self.spread();
    let value = middle + (sum / total - middle) * total / squares.sqrt();
    ((value - low) / (high - low)).clamp(0.0, 1.0)
  }

  /// The middle of one octave's folded noise, and where its lowest and highest 1% begin, which are spread to 0 and 1.
  /// Measured, not derived: gradient noise has no tidy distribution.
  fn spread(self) -> (f32, f32, f32) {
    match self {
      Self::Smooth => (0.0, -0.4, 0.4),
      Self::Turbulent => (0.12, 0.0, 0.43),
      Self::Ridged => (0.77, 0.33, 1.0),
    }
  }
}

/// What takes a point in a unit box `unit_box` across in scene units to noise cells 1 across, for cells `scale` of the
/// box's shorter side. Cells stay square however the box is stretched.
pub(crate) fn to_cells(unit_box: Vec2, scale: f32) -> Vec2 {
  unit_box / (unit_box.min_element().max(f32::EPSILON) * scale.max(1e-3))
}

/// Octaves a noise sums when a file doesn't say
pub(crate) fn default_octaves() -> u32 {
  3
}

/// Gradient (Perlin) noise at `p`, about -1 to 1, repeating every `period` cells along z
///
/// Perlin rather than simplex, as its lattice repeats along an axis just by wrapping the cell index, which is what
/// lets evolution loop.
fn gradient_noise(p: Vec3, period: i32, seed: u32) -> f32 {
  let cell = p.floor();
  let f = p - cell;
  let (x, y, z) = (cell.x as i32, cell.y as i32, cell.z as i32);
  let corner = |dx: i32, dy: i32, dz: i32| {
    let hash = hash(x + dx, y + dy, (z + dz).rem_euclid(period), seed);
    gradient(hash, f - Vec3::new(dx as f32, dy as f32, dz as f32))
  };
  let (u, v, w) = (fade(f.x), fade(f.y), fade(f.z));
  let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
  lerp(
    lerp(
      lerp(corner(0, 0, 0), corner(1, 0, 0), u),
      lerp(corner(0, 1, 0), corner(1, 1, 0), u),
      v,
    ),
    lerp(
      lerp(corner(0, 0, 1), corner(1, 0, 1), u),
      lerp(corner(0, 1, 1), corner(1, 1, 1), u),
      v,
    ),
    w,
  )
}

/// Ken Perlin's quintic ease, so the noise is smooth across cell edges
fn fade(t: f32) -> f32 {
  t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// `d` dotted with a gradient pointing anywhere on the sphere, picked by `hash`
///
/// Any direction, rather than Perlin's 12 toward a cube's edges: on a 2D slice most of those lie along x or y, which
/// shows as streaks and right angles at the low resolutions this is drawn at.
fn gradient(hash: u32, d: Vec3) -> f32 {
  // even over the sphere: z even from -1 to 1, then a turn around it
  let z = (hash >> 16) as f32 / 32_767.5 - 1.0;
  let (sin, cos) = ((hash & 0xffff) as f32 * (std::f32::consts::TAU / 65_536.0)).sin_cos();
  let r = (1.0 - z * z).sqrt();
  Vec3::new(r * cos, r * sin, z).dot(d)
}

/// An integer hash of a lattice point, so the noise is the same everywhere it's drawn
pub(crate) fn hash(x: i32, y: i32, z: i32, seed: u32) -> u32 {
  let mut h = seed.wrapping_mul(0x9e37_79b9);
  for part in [x, y, z] {
    h = (h ^ part as u32).wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
  }
  h
}

#[cfg(test)]
mod tests {
  use super::*;

  const STYLES: [NoiseStyle; 3] = [
    NoiseStyle::Smooth,
    NoiseStyle::Turbulent,
    NoiseStyle::Ridged,
  ];

  fn grid() -> impl Iterator<Item = Vec2> {
    (0..40).flat_map(|x| (0..40).map(move |y| Vec2::new(x as f32, y as f32) * 0.173))
  }

  #[test]
  fn stays_between_zero_and_one() {
    for style in STYLES {
      for p in grid() {
        let n = style.sample(p, 0.3, 4, 1);
        assert!((0.0..=1.0).contains(&n), "{style:?} gave {n} at {p}");
      }
    }
  }

  #[test]
  fn evolution_loops_at_every_whole_number() {
    for style in STYLES {
      for p in grid().take(100) {
        let start = style.sample(p, 0.25, 3, 1);
        assert!((start - style.sample(p, 1.25, 3, 1)).abs() < 1e-4);
        assert!((start - style.sample(p, -0.75, 3, 1)).abs() < 1e-4);
      }
    }
  }

  #[test]
  fn evolution_changes_it_in_place() {
    let p = Vec2::new(1.3, 2.7);
    let n = NoiseStyle::Smooth.sample(p, 0.0, 3, 1);
    assert!((n - NoiseStyle::Smooth.sample(p, 0.4, 3, 1)).abs() > 1e-3);
  }

  #[test]
  fn the_same_for_a_seed_and_different_between_seeds() {
    let p = Vec2::new(1.3, 2.7);
    let n = NoiseStyle::Smooth.sample(p, 0.0, 3, 1);
    assert_eq!(n, NoiseStyle::Smooth.sample(p, 0.0, 3, 1));
    assert_ne!(n, NoiseStyle::Smooth.sample(p, 0.0, 3, 2));
  }

  #[test]
  fn is_continuous() {
    for style in STYLES {
      for p in grid().take(200) {
        let step =
          (style.sample(p, 0.0, 3, 1) - style.sample(p + Vec2::splat(0.001), 0.0, 3, 1)).abs();
        assert!(step < 0.05, "{style:?} jumped {step} at {p}");
      }
    }
  }

  #[test]
  fn uses_most_of_its_range() {
    for style in STYLES {
      let (low, high) = grid().fold((1.0f32, 0.0f32), |(low, high), p| {
        let n = style.sample(p, 0.0, 3, 1);
        (low.min(n), high.max(n))
      });
      assert!(
        low < 0.2 && high > 0.8,
        "{style:?} only spans {low} to {high}"
      );
    }
  }
}
