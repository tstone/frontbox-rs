use fast_protocol::Color;

use crate::AddressableLed;

#[derive(Debug, Clone)]
pub struct LedDeclarations {
  pub pairings: Vec<(AddressableLed, Option<Color>)>,
  pub z_index: i8,
}

impl LedDeclarations {
  pub fn new(pairings: Vec<(AddressableLed, Option<Color>)>, z_index: i8) -> Self {
    Self { pairings, z_index }
  }

  /// Update all defined colors to the provided color, leaving any uncolored LEDs unchanged.
  pub fn recolor(self, color: Color) -> Self {
    let recolored_pairings = self
      .pairings
      .into_iter()
      .map(|(led, old_color)| match old_color {
        Some(_) => (led, Some(color)),
        None => (led, None),
      })
      .collect();
    Self {
      pairings: recolored_pairings,
      z_index: self.z_index,
    }
  }
}

impl From<(AddressableLed, Option<Color>)> for LedDeclarations {
  fn from(pairing: (AddressableLed, Option<Color>)) -> Self {
    LedDeclarations::new(vec![pairing], 0)
  }
}

#[derive(Debug, Clone)]
pub struct LedIdentifications {
  pub leds: Vec<AddressableLed>,
  pub z_index: i8,
}

impl LedIdentifications {
  pub fn new(leds: Vec<AddressableLed>, z_index: i8) -> Self {
    Self { leds, z_index }
  }
}

impl From<AddressableLed> for LedIdentifications {
  fn from(led: AddressableLed) -> Self {
    LedIdentifications::new(vec![led], 0)
  }
}

impl From<(AddressableLed, Option<i8>)> for LedIdentifications {
  fn from((led, z_index): (AddressableLed, Option<i8>)) -> Self {
    LedIdentifications::new(vec![led], z_index.unwrap_or(0))
  }
}
