use crate::animation::Lerp;
use crate::led::color_sequence::*;
use crate::prelude::*;

/// # Color Sequence
///
/// A color sequence is a way to describe a series of colors without knowing exactly how many colors you need in total. For example "fade from red to blue" is a color sequence. A color sequence can be resolved into a specific set of colors by being given a concrete quantity.
/// ```rust
/// let seq = ColorSequence::fade(Rgba::red(), Rgba::blue());
///
/// // generates 6 colors, linearly interpolated from red to blue
/// let colors: Vec<Rgba<u8>> = seq.generate(6);
/// // colors[3] is probably purple-ish
/// ```
/// As shown, colors sequences are not just a list of colors, though it could do that, but instead contain a description colors including the base fill, a defined area for that fill, and alterations layered over top.
///
/// ### Fill Types
///
/// - **Pattern** - Defines an optionally repeating, fixed pattern. e.g. "red, white, blue three times"
/// - **Gradient** - Defines a linear fade between N colors
///
/// ```rust
/// // everything is red
/// ColorSequence::solid(Rgba::red())
///
/// // 2 point gradient
/// ColorSequence::fade(Rgba::red(), Rgba::blue())
///
/// // Three point gradient with given color as the center point, and hue arc of the given degrees
/// // This produces a red to orange to yellow gradient
/// ColorSequence::analogous(Rgba::orange(), 60.0)
///
/// // Three point gradient with the given lightness range, with the given color as the center point
/// // This produces a pink to red to dark red gradient
/// ColorSequence::monochromatic(Rgba::red(), 0.8)
///
/// // Complex multi-stop gradient
/// ColorSequence::gradient(vec![
///   GradientStop::new(Rgba::red(), Extent::zero()),
///   GradientStop::new(Rgba::magenta(), Extent::relative(0.35)),
///   GradientStop::new(Rgba::blue(), Extent::full()),
/// ])
///
/// // red, white, and blue, exactly three times
/// ColorSequence::pattern(
///   vec![Rgba::red(), Rgba::white(), Rgba::blue()],
///   Cycle::Times(3)
/// );
///
/// // Forever repeating pattern
/// ColorSequence::tile(vec![Rgba::red(), Rgba::white()])
/// ```
///
/// ### Fill Area
///
/// Color sequence fills can also be offset or length-constrained and aligned.
///
/// ```rust
/// // skip the outer 2 pixels
/// let seq = ColorSequence::solid(Rgba::red())
///   .padded(Extent::absolute(1), Extent:: absolute(1));
/// let colors = seq.generate(3);
/// // Result: vec![Rgba::default(), Rgba::red(), Rgba::default()]
///
/// // render only half of the total length, center-aligned
/// let seq = ColorSequence::solid(Rgba::red())
///   .anchored(Anchor::Center, Extent::relative(0.5));
/// let colors = seq.generate(4);
/// // Result: vec![Rgba::default(), Rgba::red(), Rgba::red(), Rgba::default()]
/// ```
///
/// Modifying the fill area is useful for creating progress bar-like effects.
///
/// ```rust
/// // red to blue gradient progress bar, left aligned
/// let seq = ColorSequence::fade(Rgba::red(), Rgba::blue())
///   .anchored(Anchor::Left, Extent::relative(percent_complete));
/// ```
///
/// ### Alterations
///
/// Alterations are chained onto a ColorSequence by way of `alter`. More than one alteration can be applied to a color sequence.
///
/// ```rust
/// let seq = ColorSequence::fade(Rgba::purple(), Rgba::white())
///   .rotate(180.0)
///   .reverse();
/// ```
///
/// - **Reversed** - Applies color sequence in opposite order
/// - **Rotated** - Positive degree shifts clockwise, negative degree shifts counter-clockwise
/// - **Shuffle** - Randomly re-order sequence
/// - **Overwrite** - Overwrite base fill with a child fill
///
/// #### Extents
///
/// Because color sequences are _relative_, when specifying numerical values like offsets or lengths, these are given an [Extent].
#[derive(Clone, Debug)]
pub struct ColorSequence {
  pub fill: Fill1d,
  pub fill_area: Fill1dArea,
  pub alterations: Vec<ColorSequenceAlteration>,
}

impl Default for ColorSequence {
  fn default() -> Self {
    Self {
      fill: Fill1d::Pattern {
        pattern: vec![Rgba::default()],
        cycle: Cycle::Forever,
      },
      fill_area: Fill1dArea::Full,
      alterations: Vec::new(),
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

  pub fn fill(mut self, fill: Fill1d) -> Self {
    self.fill = fill;
    self
  }

  pub fn area(mut self, area: Fill1dArea) -> Self {
    self.fill_area = area;
    self
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

  pub fn anchored(mut self, anchor: Anchor1d, length: impl Into<Extent<u16>>) -> Self {
    self.fill_area = Fill1dArea::Anchored {
      length: length.into(),
      anchor,
    };
    self
  }

  pub fn reverse(mut self) -> Self {
    self.alterations.push(ColorSequenceAlteration::Reverse);
    self
  }

  pub fn rotate(mut self, angle: f32) -> Self {
    self
      .alterations
      .push(ColorSequenceAlteration::Rotate(Extent::Relative(angle)));
    self
  }

  pub fn shuffle(mut self, seed: u64) -> Self {
    self
      .alterations
      .push(ColorSequenceAlteration::Shuffle(seed));
    self
  }

  /// Overwrite an inner fill over top of the 'base' ColorSequence
  pub fn overwrite(mut self, fill: Fill1d, area: Fill1dArea) -> Self {
    self
      .alterations
      .push(ColorSequenceAlteration::Overwrite(fill, area));
    self
  }

  pub fn alter(&mut self, alteration: ColorSequenceAlteration) {
    self.alterations.push(alteration);
  }

  /// Generate the sequence for the given quantity
  pub fn generate(&self, qty: usize) -> Vec<Rgba<u8>> {
    // render base starting fill
    let mut seq = vec![Rgba::default(); qty];
    fill1d::render_into(&mut seq, &self.fill, &self.fill_area);

    // apply modifications
    for m in &self.alterations {
      match m {
        ColorSequenceAlteration::Reverse => alteration::reverse(&mut seq),
        ColorSequenceAlteration::Rotate(rotation) => alteration::rotate(&mut seq, *rotation),
        ColorSequenceAlteration::Overwrite(fill, area) => fill1d::render_into(&mut seq, fill, area),
        ColorSequenceAlteration::Shuffle(seed) => alteration::shuffle(&mut seq, *seed),
      }
    }

    log::trace!(target: "frontbox::color", "ColorSequence: Generated color sequence: {:?}", seq);
    seq
  }
}

impl Lerp for ColorSequence {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    ColorSequence {
      fill: self.fill.interpolate(&other.fill, t),
      fill_area: self.fill_area.interpolate(&other.fill_area, t),
      alterations: if t < 0.5 {
        self.alterations.clone()
      } else {
        other.alterations.clone()
      },
    }
  }
}

impl From<Rgba<u8>> for ColorSequence {
  fn from(value: Rgba<u8>) -> Self {
    ColorSequence::solid(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn solid() {
    let cs = ColorSequence::solid(Rgba::red());
    let colors = cs.generate(3);

    assert_eq!(colors[0], Rgba::red());
    assert_eq!(colors[1], Rgba::red());
    assert_eq!(colors[2], Rgba::red());
  }

  #[test]
  fn pattern() {
    let cs = ColorSequence::pattern(vec![Rgba::red(), Rgba::white()], Cycle::Times(2));
    let colors = cs.generate(5);

    assert_eq!(colors[0], Rgba::red());
    assert_eq!(colors[1], Rgba::white());
    assert_eq!(colors[2], Rgba::red());
    assert_eq!(colors[3], Rgba::white());
    assert_eq!(colors[4], Rgba::default());
  }
}
