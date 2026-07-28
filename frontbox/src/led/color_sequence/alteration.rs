use crate::led::color_sequence::*;
use crate::prelude::*;
use rand::rngs::SmallRng;
use rand::{SeedableRng, seq::SliceRandom};

#[derive(Debug, Clone)]
pub enum ColorSequenceAlteration {
  Reverse,
  Rotate(Extent<i16>),
  Overwrite(Fill1d, Fill1dArea),
  Shuffle(u64),
}

pub(crate) fn reverse(seq: &mut Vec<Rgba<u8>>) {
  seq.reverse();
}

pub(crate) fn rotate(seq: &mut Vec<Rgba<u8>>, rotation: Extent<i16>) {
  let len = seq.len();
  let (left, steps) = match rotation {
    Extent::Absolute(steps) => (steps < 0, steps.abs() as usize),
    Extent::Relative(percent) => (
      percent < 0.0,
      (percent.abs() / 360.0 * len as f32).round() as usize % len,
    ),
  };

  if steps != 0 {
    if left {
      seq.rotate_left(steps);
    } else {
      seq.rotate_right(steps);
    }
  }
}

pub(crate) fn shuffle(seq: &mut Vec<Rgba<u8>>, seed: u64) {
  let mut rng = SmallRng::seed_from_u64(seed);
  seq.shuffle(&mut rng);
}
