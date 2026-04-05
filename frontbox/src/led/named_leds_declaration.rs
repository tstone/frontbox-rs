use fast_protocol::Color;

use crate::prelude::{Context, LedDeclarations, LedIdentifications};
use crate::{AddressableLed, HardwareSelection};

pub fn named_leds(ctx: Context, names: Vec<&str>) -> MultipleLedDeclarations {
  let mut leds = Vec::new();
  for name in names {
    let ill = ctx.illuminations.get(name).expect("LED not found");
    leds.push((
      ill
        .leds
        .first()
        .expect("LED has no addressable LEDs")
        .clone(),
      None,
    ));
  }

  MultipleLedDeclarations {
    pairings: leds,
    z_index: None,
  }
}

pub fn selected_leds(ctx: &Context, sel: HardwareSelection) -> MultipleLedDeclarations {
  let illums = sel.get_illuminations(ctx);

  let mut leds = Vec::new();
  for ill in illums {
    for led in &ill.leds {
      leds.push((led.clone(), None));
    }
  }

  MultipleLedDeclarations {
    pairings: leds,
    z_index: None,
  }
}

pub struct MultipleLedDeclarations {
  pairings: Vec<(AddressableLed, Option<Color>)>,
  z_index: Option<i8>,
}

impl MultipleLedDeclarations {
  /// Set all LEDs to the same color
  pub fn color_all(mut self, color: Color) -> MultipleLedDeclarations {
    for (_, c) in self.pairings.iter_mut() {
      *c = Some(color);
    }
    self
  }

  /// Set a single LED to a color, identified by its index in the original list of names
  pub fn color_idx(mut self, index: usize, color: Color) -> MultipleLedDeclarations {
    self.pairings.get_mut(index).expect("Index out of bounds").1 = Some(color);
    self
  }

  /// Set the LEDs to alternate between the provided colors in order
  pub fn alternate(mut self, colors: Vec<Color>) -> MultipleLedDeclarations {
    for (i, (_, c)) in self.pairings.iter_mut().enumerate() {
      *c = Some(colors[i % colors.len()]);
    }
    self
  }

  /// Create a gradient across all LEDs from the from color to the to color
  pub fn gradient(mut self, from: Color, to: Color) -> MultipleLedDeclarations {
    let n = self.pairings.len();
    for (i, (_, c)) in self.pairings.iter_mut().enumerate() {
      let t = if n == 1 {
        0.0
      } else {
        i as f32 / (n - 1) as f32
      };
      *c = Some(from.mix(&to, t));
    }
    self
  }

  /// Rotate the LED-color pairings to the right by the specified number of steps
  pub fn rotate_right(mut self, deg: f32) -> MultipleLedDeclarations {
    let n = self.pairings.len();
    let steps = (deg / 360.0 * n as f32).round() as usize % n;
    self.pairings.rotate_right(steps);
    self
  }

  /// Rotate the LED-color pairings to the left by the specified number of steps
  pub fn rotate_left(mut self, deg: f32) -> MultipleLedDeclarations {
    let n = self.pairings.len();
    let steps = (deg / 360.0 * n as f32).round() as usize % n;
    self.pairings.rotate_left(steps);
    self
  }

  pub fn z_index(mut self, z: i8) -> Self {
    self.z_index = Some(z);
    self
  }
}

impl From<MultipleLedDeclarations> for LedDeclarations {
  fn from(decl: MultipleLedDeclarations) -> Self {
    LedDeclarations::new(decl.pairings, decl.z_index.unwrap_or(0))
  }
}

impl From<MultipleLedDeclarations> for LedIdentifications {
  fn from(decl: MultipleLedDeclarations) -> Self {
    LedIdentifications::new(
      decl.pairings.into_iter().map(|(led, _)| led).collect(),
      decl.z_index.unwrap_or(0),
    )
  }
}
