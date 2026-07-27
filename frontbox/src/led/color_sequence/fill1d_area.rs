use crate::animation::Lerp;
use crate::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum Fill1dArea {
  #[default]
  Full,
  Padded {
    left: Extent<u16>,
    right: Extent<u16>,
  },
  Anchored {
    length: Extent<u16>,
    anchor: Anchor,
  },
}

impl Fill1dArea {
  pub fn left_padding_mut(&mut self) -> Option<&mut Extent<u16>> {
    match self {
      Self::Padded { left, .. } => Some(left),
      _ => None,
    }
  }

  pub fn right_padding_mut(&mut self) -> Option<&mut Extent<u16>> {
    match self {
      Self::Padded { right, .. } => Some(right),
      _ => None,
    }
  }

  pub fn anchor_length_mut(&mut self) -> Option<&mut Extent<u16>> {
    match self {
      Self::Anchored { length, .. } => Some(length),
      _ => None,
    }
  }

  pub fn anchor_mut(&mut self) -> Option<&mut Anchor> {
    match self {
      Self::Anchored { anchor, .. } => Some(anchor),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Anchor {
  #[default]
  Start,
  Center,
  End,
}

impl Lerp for Fill1dArea {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Fill1dArea::Full, Fill1dArea::Full) => Fill1dArea::Full,
      (
        Fill1dArea::Padded {
          left: la,
          right: ra,
        },
        Fill1dArea::Padded {
          left: lb,
          right: rb,
        },
      ) => Fill1dArea::Padded {
        left: la.interpolate(lb, t),
        right: ra.interpolate(rb, t),
      },
      (
        Fill1dArea::Anchored {
          length: len_a,
          anchor,
        },
        Fill1dArea::Anchored { length: len_b, .. },
      ) => Fill1dArea::Anchored {
        length: len_a.interpolate(len_b, t),
        anchor: *anchor,
      },
      _ => {
        if t < 0.5 {
          self.clone()
        } else {
          other.clone()
        }
      }
    }
  }
}
