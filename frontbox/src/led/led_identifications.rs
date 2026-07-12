use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LedIdentifications {
  pub leds: Vec<AddressableLed>,
  pub z_index: i8,
}

impl LedIdentifications {
  pub fn new(leds: Vec<AddressableLed>, z_index: i8) -> Self {
    Self { leds, z_index }
  }

  pub fn at_z(mut self, z_index: i8) -> Self {
    self.z_index = z_index;
    self
  }
}

impl From<AddressableLed> for LedIdentifications {
  fn from(led: AddressableLed) -> Self {
    LedIdentifications::new(vec![led], 0)
  }
}

impl From<Vec<AddressableLed>> for LedIdentifications {
  fn from(leds: Vec<AddressableLed>) -> Self {
    LedIdentifications::new(leds, 0)
  }
}

impl From<(AddressableLed, Option<i8>)> for LedIdentifications {
  fn from((led, z_index): (AddressableLed, Option<i8>)) -> Self {
    LedIdentifications::new(vec![led], z_index.unwrap_or(0))
  }
}

pub trait LedIdentificationsExt {
  /// Set the z-index
  fn at_z(self, z_index: i8) -> LedIdentifications;
}

impl LedIdentificationsExt for AddressableLed {
  fn at_z(self, z_index: i8) -> LedIdentifications {
    let ids: LedIdentifications = self.into();
    ids.at_z(z_index)
  }
}

impl LedIdentificationsExt for Vec<AddressableLed> {
  fn at_z(self, z_index: i8) -> LedIdentifications {
    let ids: LedIdentifications = self.into();
    ids.at_z(z_index)
  }
}
