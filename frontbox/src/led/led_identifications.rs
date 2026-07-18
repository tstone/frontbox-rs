use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LedIdentifications {
  pub leds: Vec<LedAddress>,
  pub z_index: i8,
}

impl LedIdentifications {
  pub fn new(leds: Vec<LedAddress>, z_index: i8) -> Self {
    Self { leds, z_index }
  }

  pub fn at_z(mut self, z_index: i8) -> Self {
    self.z_index = z_index;
    self
  }
}

impl Contextual<LedIdentifications> for HardwareQuery {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    let addresses = self.get_leds_addresses(&ctx);
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for Vec<HardwareQuery> {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    let addresses = self
      .iter()
      .flat_map(|q| q.get_leds_addresses(ctx))
      .collect();
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for Vec<&HardwareQuery> {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    let addresses = self
      .iter()
      .flat_map(|q| q.get_leds_addresses(ctx))
      .collect();
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for LedIdentifications {
  fn resolve(&self, _ctx: &Context) -> LedIdentifications {
    self.clone()
  }
}

impl Contextual<LedIdentifications> for Box<dyn Contextual<LedIdentifications>> {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    (**self).resolve(ctx)
  }
}

impl Contextual<LedIdentifications> for &Box<dyn Contextual<LedIdentifications>> {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    (**self).resolve(ctx)
  }
}
