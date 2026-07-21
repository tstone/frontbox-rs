use crate::led::color_sequence::*;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ColorSequence {
  pub fill: Fill,
  pub fill_area: FillArea,
  pub modifications: Vec<Modification>,
}

impl Default for ColorSequence {
  fn default() -> Self {
    Self {
      fill: Fill::Pattern {
        pattern: vec![Rgba::default()],
        cycle: Cycle::Forever,
      },
      fill_area: FillArea::Full,
      modifications: Vec::new(),
    }
  }
}

impl ColorSequence {
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
      fill: Fill::Gradient { stops },
      ..Default::default()
    }
  }

  pub fn solid(color: Rgba<u8>) -> Self {
    Self::pattern(vec![color], Cycle::Forever)
  }

  pub fn pattern(pattern: Vec<Rgba<u8>>, cycle: Cycle) -> Self {
    ColorSequence {
      fill: Fill::Pattern { pattern, cycle },
      ..Default::default()
    }
  }

  /// Repeat a pattern forever
  pub fn tile(pattern: Vec<Rgba<u8>>) -> Self {
    ColorSequence {
      fill: Fill::Pattern {
        pattern,
        cycle: Cycle::Forever,
      },
      ..Default::default()
    }
  }

  pub fn padded(mut self, padding_left: Extent, padding_right: Extent) -> Self {
    self.fill_area = FillArea::Padded {
      left: padding_left,
      right: padding_right,
    };
    self
  }

  pub fn padding_left(mut self, padding: Extent) -> Self {
    self.fill_area = match self.fill_area {
      FillArea::Padded { right, .. } => FillArea::Padded {
        left: padding,
        right,
      },
      _ => FillArea::Padded {
        left: padding,
        right: Extent::full(),
      },
    };
    self
  }

  pub fn padding_right(mut self, padding: Extent) -> Self {
    self.fill_area = match self.fill_area {
      FillArea::Padded { left, .. } => FillArea::Padded {
        left,
        right: padding,
      },
      _ => FillArea::Padded {
        left: Extent::zero(),
        right: padding,
      },
    };
    self
  }

  pub fn anchored(mut self, anchor: Anchor, length: Extent) -> Self {
    self.fill_area = FillArea::Anchored { length, anchor };
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
    fill::render_into(&mut seq, &self.fill, &self.fill_area);

    // apply modifications
    for m in &self.modifications {
      match m {
        Modification::Reversed => modification::reverse(&mut seq),
        Modification::Rotated { rotation } => modification::rotate(&mut seq, *rotation),
        Modification::InnerFill { fill, area } => fill::render_into(&mut seq, fill, area),
        Modification::Shuffle { seed } => modification::shuffle(&mut seq, *seed),
      }
    }

    seq
  }
}
