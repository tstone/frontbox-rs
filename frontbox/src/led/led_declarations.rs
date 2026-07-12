use crate::AddressableLed;
use image::Rgba;

#[derive(Debug, Clone)]
pub struct LedDeclarations {
  // TODO: this is practically identical to MultipleLedDeclarations
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
