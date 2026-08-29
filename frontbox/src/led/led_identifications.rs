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

impl Contextual<LedIdentifications> for LedQuery {
  fn resolve(&self, ctx: &SystemContext) -> LedIdentifications {
    let addresses = self.query_addresses(ctx.into());
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for Vec<LedQuery> {
  fn resolve(&self, ctx: &SystemContext) -> LedIdentifications {
    let addresses = self
      .iter()
      .flat_map(|q| q.query_addresses(ctx.into()))
      .collect();
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for Vec<&LedQuery> {
  fn resolve(&self, ctx: &SystemContext) -> LedIdentifications {
    let addresses = self
      .iter()
      .flat_map(|q| q.query_addresses(ctx.into()))
      .collect();
    LedIdentifications::new(addresses, 0)
  }
}

impl Contextual<LedIdentifications> for LedIdentifications {
  fn resolve(&self, _ctx: &SystemContext) -> LedIdentifications {
    self.clone()
  }
}

impl Contextual<LedIdentifications> for Box<dyn Contextual<LedIdentifications> + Send + Sync> {
  fn resolve(&self, ctx: &SystemContext) -> LedIdentifications {
    (**self).resolve(ctx)
  }
}

impl<C, T> Contextual<T> for &C
where
  C: Contextual<T> + ?Sized,
{
  fn resolve(&self, ctx: &SystemContext) -> T {
    (**self).resolve(ctx)
  }
}
