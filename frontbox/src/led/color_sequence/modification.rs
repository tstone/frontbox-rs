use crate::led::color_sequence::*;
use crate::prelude::*;
use rand::rngs::SmallRng;
use rand::{SeedableRng, seq::SliceRandom};

#[derive(Debug, Clone)]
pub enum Modification {
  Reversed,
  Rotated { rotation: f32 },
  Shuffle { seed: u64 },
  InnerFill { fill: Fill, area: FillArea },
}

impl Modification {
  pub fn reversed() -> Self {
    Self::Reversed
  }

  pub fn rotated(rotation: f32) -> Self {
    Self::Rotated { rotation }
  }

  pub fn inner_fill(fill: Fill, area: FillArea) -> Self {
    Self::InnerFill { fill, area }
  }

  pub fn shuffle(seed: u64) -> Self {
    Self::Shuffle { seed }
  }

  pub fn transparent_at(extant: Extent<u16>) -> Self {
    Self::inner_fill(
      Fill::Pattern {
        pattern: vec![Rgba::default()],
        cycle: Cycle::Once,
      },
      FillArea::Padded {
        left: extant,
        right: Extent::full(),
      },
    )
  }

  pub fn rotation_mut(&mut self) -> Option<&mut f32> {
    match self {
      Self::Rotated { rotation } => Some(rotation),
      _ => None,
    }
  }

  pub fn inner_fill_mut(&mut self) -> Option<&mut Fill> {
    match self {
      Self::InnerFill { fill, .. } => Some(fill),
      _ => None,
    }
  }

  pub fn inner_fill_area_mut(&mut self) -> Option<&mut FillArea> {
    match self {
      Self::InnerFill { area, .. } => Some(area),
      _ => None,
    }
  }
}

pub(crate) fn reverse(seq: &mut Vec<Rgba<u8>>) {
  seq.reverse();
}

pub(crate) fn rotate(seq: &mut Vec<Rgba<u8>>, rotation: f32) {
  if rotation != 0.0 {
    let len = seq.len();
    let steps = (rotation.abs() / 360.0 * len as f32).round() as usize % len;

    // negative rotation = counterclockwise
    if rotation < 0.0 {
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
