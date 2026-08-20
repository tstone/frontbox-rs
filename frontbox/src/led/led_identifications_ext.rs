use crate::prelude::*;

pub trait LedIdentificationsExt:
  Contextual<LedIdentifications> + Send + Sync + Sized + 'static
{
  fn at_z(self, z_index: i8) -> IdentificationAtZ {
    IdentificationAtZ {
      other: Box::new(self),
      z: z_index,
    }
  }
}

impl<T: Contextual<LedIdentifications> + Sized + Send + Sync + 'static> LedIdentificationsExt
  for T
{
}

#[derive(Clone)]
pub struct IdentificationAtZ {
  other: Box<dyn Contextual<LedIdentifications> + Send + Sync>,
  z: i8,
}

impl Contextual<LedIdentifications> for IdentificationAtZ {
  fn resolve(&self, ctx: &SystemContext) -> LedIdentifications {
    let ids = self.other.resolve(ctx);
    ids.at_z(self.z)
  }
}
