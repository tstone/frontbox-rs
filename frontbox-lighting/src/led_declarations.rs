use frontbox_core::hardware::LedAddress;
use image::Rgba;

#[derive(Debug, Clone)]
pub struct LedDeclarations<'a> {
  pub pairings: Vec<(&'a LedAddress, Rgba<u8>)>,
  pub z_index: i8,
  /// The colors flash on and off rather than staying on
  pub flashing: bool,
}

impl<'a> LedDeclarations<'a> {
  pub fn new(pairings: Vec<(&'a LedAddress, Rgba<u8>)>, z_index: i8) -> Self {
    Self {
      pairings,
      z_index,
      flashing: false,
    }
  }

  /// Flash the colors on and off, in step with every other flashing declaration
  pub fn flashing(self) -> Self {
    Self { flashing: true, ..self }
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
      ..self
    }
  }
}

impl<'a> From<(&'a LedAddress, Rgba<u8>)> for LedDeclarations<'a> {
  fn from(pairing: (&'a LedAddress, Rgba<u8>)) -> Self {
    LedDeclarations::new(vec![pairing], 0)
  }
}
