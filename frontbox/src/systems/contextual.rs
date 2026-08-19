use dyn_clone::DynClone;

use crate::prelude::SystemContext;

pub trait Contextual<T>: DynClone {
  fn resolve(&self, ctx: &SystemContext) -> T;
}

dyn_clone::clone_trait_object!(<T> Contextual<T>);
