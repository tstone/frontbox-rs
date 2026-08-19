use crate::prelude::SystemContext;

pub trait Contextual<T> {
  fn resolve(&self, ctx: &SystemContext) -> T;
}
