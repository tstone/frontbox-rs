use crate::animation::Lerp;
use crate::led::color_sequence::*;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Fill {
  /// Linear interpolate between multiple colors
  Gradient { stops: Vec<GradientStop> },
  /// Apply an explicit pattern with a variable repeat
  Pattern {
    pattern: Vec<Rgba<u8>>,
    cycle: Cycle,
  },
}

impl Fill {
  pub fn gradient_stops_mut(&mut self) -> Option<&mut Vec<GradientStop>> {
    match self {
      Self::Gradient { stops } => Some(stops),
      _ => None,
    }
  }

  pub fn pattern_mut(&mut self) -> Option<&mut Vec<Rgba<u8>>> {
    match self {
      Self::Pattern { pattern, .. } => Some(pattern),
      _ => None,
    }
  }

  pub fn cycle_mut(&mut self) -> Option<&mut Cycle> {
    match self {
      Self::Pattern { cycle, .. } => Some(cycle),
      _ => None,
    }
  }

  /// Re-colors the entire fill to a single color
  pub fn recolor(&mut self, color: Rgba<u8>) {
    match self {
      Self::Gradient { stops } => {
        for stop in stops {
          stop.color = color.clone();
        }
      }
      Self::Pattern { pattern, .. } => {
        *pattern = vec![color; pattern.len()];
      }
    }
  }

  /// Re-color a specific point in the fill
  /// For gradient this changes the color of that stop
  /// For pattern this changes the color at that index
  pub fn recolor_at(&mut self, index: usize, color: Rgba<u8>) {
    match self {
      Self::Gradient { stops } => {
        if index < stops.len() {
          stops[index].color = color;
        }
      }
      Self::Pattern { pattern, .. } => {
        if index < pattern.len() {
          pattern[index] = color
        }
      }
    }
  }
}

impl Lerp for Fill {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Fill::Gradient { stops: a }, Fill::Gradient { stops: b }) if a.len() == b.len() => {
        Fill::Gradient {
          stops: a
            .iter()
            .zip(b)
            .map(|(sa, sb)| sa.interpolate(sb, t))
            .collect(),
        }
      }
      (Fill::Pattern { pattern: a, cycle }, Fill::Pattern { pattern: b, .. })
        if a.len() == b.len() =>
      {
        Fill::Pattern {
          pattern: a
            .iter()
            .zip(b)
            .map(|(ca, cb)| ca.interpolate(cb, t))
            .collect(),
          cycle: cycle.clone(),
        }
      }
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

pub(crate) fn render_into(seq: &mut Vec<Rgba<u8>>, fill: &Fill, area: &FillArea) {
  let starting_len = seq.len() as u16;

  // calculate actual fill length based on area setting
  let fill_len = match area {
    FillArea::Full => starting_len,
    FillArea::Padded { left, right } => {
      starting_len - left.to_absolute(starting_len) - right.to_absolute(starting_len)
    }
    FillArea::Anchored { length: l, .. } => l.to_absolute(starting_len),
  };

  // generate fill
  let fill = match fill {
    Fill::Gradient { stops } => gradient::render(stops, fill_len),
    Fill::Pattern { pattern, cycle } => pattern::render(pattern, *cycle, fill_len),
  };

  // copy fill into offset position
  let left = match area {
    FillArea::Full => 0,
    FillArea::Padded { left, .. } => left.to_absolute(starting_len),
    FillArea::Anchored { anchor, .. } => match anchor {
      Anchor::Start => 0,
      Anchor::End => starting_len - fill_len,
      Anchor::Center => (starting_len - fill_len) / 2,
    },
  };

  for (i, pixel) in fill.iter().enumerate() {
    seq[i + left as usize] = *pixel;
  }
}
