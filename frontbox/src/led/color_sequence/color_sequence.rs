use crate::animation::Lerp;
use crate::led::color_sequence::*;
use crate::prelude::*;

/// A 1d description of a sequence of colors
#[derive(Clone, Debug)]
pub struct ColorSequence {
  pub fill: Fill1d,
  pub fill_area: Fill1dArea,
  pub modifications: Vec<Modification>,
}

impl Default for ColorSequence {
  fn default() -> Self {
    Self {
      fill: Fill1d::Pattern {
        pattern: vec![Rgba::default()],
        cycle: Cycle::Forever,
      },
      fill_area: Fill1dArea::Full,
      modifications: Vec::new(),
    }
  }
}

impl ColorSequence {
  pub fn off() -> Self {
    Self::tile(vec![Rgba::default()])
  }

  pub fn fade(from: Rgba<u8>, to: Rgba<u8>) -> Self {
    Self::gradient(vec![
      GradientStop {
        color: from,
        position: Extent::zero(),
      },
      GradientStop {
        color: to,
        position: Extent::full(),
      },
    ])
  }

  pub fn monochromatic(root: Rgba<u8>, range: f32) -> Self {
    Self::gradient(vec![
      GradientStop {
        color: root.with_lightness(0.5 - (range / 2.0)),
        position: Extent::zero(),
      },
      GradientStop {
        color: root.with_lightness(0.5),
        position: Extent::Relative(0.5),
      },
      GradientStop {
        color: root.with_lightness(0.5 + (range / 2.0)),
        position: Extent::full(),
      },
    ])
  }

  pub fn analogous(root: Rgba<u8>, degree: f32) -> Self {
    Self::gradient(vec![
      GradientStop {
        color: root.hue_shift(-degree),
        position: Extent::zero(),
      },
      GradientStop {
        color: root,
        position: Extent::Relative(0.5),
      },
      GradientStop {
        color: root.hue_shift(degree),
        position: Extent::full(),
      },
    ])
  }

  pub fn gradient(stops: Vec<GradientStop>) -> Self {
    ColorSequence {
      fill: Fill1d::Gradient { stops },
      ..Default::default()
    }
  }

  pub fn solid(color: Rgba<u8>) -> Self {
    Self::pattern(vec![color], Cycle::Forever)
  }

  pub fn pattern(pattern: Vec<Rgba<u8>>, cycle: Cycle) -> Self {
    ColorSequence {
      fill: Fill1d::Pattern { pattern, cycle },
      ..Default::default()
    }
  }

  /// Repeat the given pattern exactly once
  pub fn exact(pattern: Vec<Rgba<u8>>) -> Self {
    ColorSequence {
      fill: Fill1d::Pattern {
        pattern,
        cycle: Cycle::Once,
      },
      ..Default::default()
    }
  }

  /// Repeat a pattern forever
  pub fn tile(pattern: Vec<Rgba<u8>>) -> Self {
    ColorSequence {
      fill: Fill1d::Pattern {
        pattern,
        cycle: Cycle::Forever,
      },
      ..Default::default()
    }
  }

  pub fn padded(
    mut self,
    padding_left: impl Into<Extent<u16>>,
    padding_right: impl Into<Extent<u16>>,
  ) -> Self {
    self.fill_area = Fill1dArea::Padded {
      left: padding_left.into(),
      right: padding_right.into(),
    };
    self
  }

  pub fn padding_left(mut self, padding: impl Into<Extent<u16>>) -> Self {
    self.fill_area = match self.fill_area {
      Fill1dArea::Padded { right, .. } => Fill1dArea::Padded {
        left: padding.into(),
        right,
      },
      _ => Fill1dArea::Padded {
        left: padding.into(),
        right: Extent::zero(),
      },
    };
    self
  }

  pub fn padding_right(mut self, padding: impl Into<Extent<u16>>) -> Self {
    self.fill_area = match self.fill_area {
      Fill1dArea::Padded { left, .. } => Fill1dArea::Padded {
        left,
        right: padding.into(),
      },
      _ => Fill1dArea::Padded {
        left: Extent::zero(),
        right: padding.into(),
      },
    };
    self
  }

  pub fn anchored(mut self, anchor: Anchor, length: impl Into<Extent<u16>>) -> Self {
    self.fill_area = Fill1dArea::Anchored {
      length: length.into(),
      anchor,
    };
    self
  }

  pub fn modify(mut self, modification: Modification) -> Self {
    self.modifications.push(modification);
    self
  }

  /// Generate the sequence for the given quantity
  pub fn generate(&self, qty: usize) -> Vec<Rgba<u8>> {
    // render base starting fill
    let mut seq = vec![Rgba::default(); qty];
    fill1d::render_into(&mut seq, &self.fill, &self.fill_area);

    // apply modifications
    for m in &self.modifications {
      match m {
        Modification::Reversed => modification::reverse(&mut seq),
        Modification::Rotated { rotation } => modification::rotate(&mut seq, *rotation),
        Modification::InnerFill { fill, area } => fill1d::render_into(&mut seq, fill, area),
        Modification::Shuffle { seed } => modification::shuffle(&mut seq, *seed),
      }
    }

    seq
  }
}

impl Lerp for ColorSequence {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    ColorSequence {
      fill: self.fill.interpolate(&other.fill, t),
      fill_area: self.fill_area.interpolate(&other.fill_area, t),
      modifications: if t < 0.5 {
        self.modifications.clone()
      } else {
        other.modifications.clone()
      },
    }
  }
}
