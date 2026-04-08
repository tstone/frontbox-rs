use image::Rgba;

use crate::AddressableLed;

#[derive(Debug, Clone)]
pub struct LedDeclarations {
  pub pairings: Vec<(AddressableLed, Rgba<u8>)>,
  pub z_index: i8,
}

impl LedDeclarations {
  pub fn new(pairings: Vec<(AddressableLed, Rgba<u8>)>, z_index: i8) -> Self {
    Self { pairings, z_index }
  }

  /// Update all defined colors to the provided color, leaving any uncolored LEDs unchanged.
  pub fn recolor(self, color: Rgba<u8>) -> Self {
    let recolored_pairings = self
      .pairings
      .into_iter()
      .map(|(led, _)| (led, color))
      .collect();
    Self {
      pairings: recolored_pairings,
      z_index: self.z_index,
    }
  }
}

impl From<(AddressableLed, Rgba<u8>)> for LedDeclarations {
  fn from(pairing: (AddressableLed, Rgba<u8>)) -> Self {
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
