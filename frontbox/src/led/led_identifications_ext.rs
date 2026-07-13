use crate::prelude::*;

pub trait LedIdentificationsExt: Contextual<LedIdentifications> + Sized + 'static {
  fn at_z(self, z_index: i8) -> Box<dyn Contextual<LedIdentifications>> {
    Box::new(IdentificationAtZ {
      other: Box::new(self),
      z: z_index,
    })
  }
}

impl<T: Contextual<LedIdentifications> + Sized + 'static> LedIdentificationsExt for T {}

struct IdentificationAtZ {
  other: Box<dyn Contextual<LedIdentifications>>,
  z: i8,
}

impl Contextual<LedIdentifications> for IdentificationAtZ {
  fn resolve(&self, ctx: &Context) -> LedIdentifications {
    let ids = self.other.resolve(ctx);
    ids.at_z(self.z)
  }
}
