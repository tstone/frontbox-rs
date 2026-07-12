use crate::prelude::*;

#[derive(Clone)]
pub struct LedLookup {
  illuminations: IlluminationLookup,
}

impl LedLookup {
  pub fn new(illuminations: IlluminationLookup) -> Self {
    Self { illuminations }
  }
}

impl LedLookup {
  pub fn query(&self, query: impl Into<HardwareQuery>) -> Vec<AddressableLed> {
    let query = query.into();
    self
      .illuminations
      .query(&query)
      .iter()
      .flat_map(|illum| &illum.leds)
      .cloned()
      .collect()
  }
}
