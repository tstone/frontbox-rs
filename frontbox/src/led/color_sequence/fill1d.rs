use crate::animation::Lerp;
use crate::led::color_sequence::*;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Fill1d {
  /// Linear interpolate between multiple colors
  Gradient { stops: Vec<GradientStop> },
  /// Apply an explicit pattern with a variable repeat
  Pattern {
    pattern: Vec<Rgba<u8>>,
    cycle: Cycle,
  },
}

impl Fill1d {
  pub fn solid(color: Rgba<u8>) -> Self {
    Self::Pattern {
      pattern: vec![color],
      cycle: Cycle::Forever,
    }
  }

  pub fn fade(from: Rgba<u8>, to: Rgba<u8>) -> Self {
    Self::Gradient {
      stops: vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)],
    }
  }

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

impl Lerp for Fill1d {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Fill1d::Gradient { stops: a }, Fill1d::Gradient { stops: b }) if a.len() == b.len() => {
        Fill1d::Gradient {
          stops: a
            .iter()
            .zip(b)
            .map(|(sa, sb)| sa.interpolate(sb, t))
            .collect(),
        }
      }
      (Fill1d::Pattern { pattern: a, cycle }, Fill1d::Pattern { pattern: b, .. })
        if a.len() == b.len() =>
      {
        Fill1d::Pattern {
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

pub(crate) fn render_into(seq: &mut Vec<Rgba<u8>>, fill: &Fill1d, area: &Fill1dArea) {
  let starting_len = seq.len() as u16;

  // calculate actual fill length based on area setting
  let mut fill_len = match area {
    Fill1dArea::Full => starting_len,
    Fill1dArea::Padded { left, right } => starting_len
      .saturating_sub(left.to_absolute(starting_len))
      .saturating_sub(right.to_absolute(starting_len)),
    Fill1dArea::Anchored { length: l, .. } => l.to_absolute(starting_len),
  };
  
  // it's possible for fill_len to be larger if the values input to anchored are larger
  // so cap it at the starting length
  fill_len = fill_len.min(starting_len);
  
  // generate fill
  let fill = match fill {
    Fill1d::Gradient { stops } => gradient::render(stops, fill_len),
    Fill1d::Pattern { pattern, cycle } => pattern::render(pattern, *cycle, fill_len),
  };

  // copy fill into offset position
  let left = match area {
    Fill1dArea::Full => 0,
    Fill1dArea::Padded { left, .. } => left.to_absolute(starting_len),
    Fill1dArea::Anchored { anchor, .. } => match anchor {
      Anchor1d::Start => 0,
      Anchor1d::End => starting_len - fill_len,
      Anchor1d::Center => (starting_len - fill_len) / 2,
    },
  };

  for (i, pixel) in fill.iter().enumerate() {
    let idx = i + left as usize;
    seq[idx] = *pixel;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn full() {
    let mut seq = vec![Rgba::default(); 3];
    render_into(&mut seq, &Fill1d::solid(Rgba::blue()), &Fill1dArea::Full);

    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0], Rgba::blue());
    assert_eq!(seq[1], Rgba::blue());
    assert_eq!(seq[2], Rgba::blue());
  }

  #[test]
  fn anchored_start() {
    let mut seq = vec![Rgba::default(); 3];
    render_into(&mut seq, &Fill1d::solid(Rgba::blue()), &Fill1dArea::anchored(Anchor1d::Start, 2));

    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0], Rgba::blue());
    assert_eq!(seq[1], Rgba::blue());
    assert_eq!(seq[2], Rgba::default());
  }

  #[test]
  fn anchored_start_overflow() {
    let mut seq = vec![Rgba::default(); 3];
    // anchor length is intentionaly oversized
    render_into(&mut seq, &Fill1d::solid(Rgba::blue()), &Fill1dArea::anchored(Anchor1d::Start, 6));

    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0], Rgba::blue());
    assert_eq!(seq[1], Rgba::blue());
    assert_eq!(seq[2], Rgba::blue());
  }

  #[test]
  fn anchored_end() {
    let mut seq = vec![Rgba::default(); 3];
    render_into(&mut seq, &Fill1d::solid(Rgba::blue()), &Fill1dArea::anchored(Anchor1d::End, 2));

    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0], Rgba::default());
    assert_eq!(seq[1], Rgba::blue());
    assert_eq!(seq[2], Rgba::blue());
  }

  #[test]
  fn anchored_center() {
    let mut seq = vec![Rgba::default(); 3];
    render_into(&mut seq, &Fill1d::solid(Rgba::blue()), &Fill1dArea::anchored(Anchor1d::Center, 1));

    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0], Rgba::default());
    assert_eq!(seq[1], Rgba::blue());
    assert_eq!(seq[2], Rgba::default());
  }
}