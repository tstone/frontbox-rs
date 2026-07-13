use crate::prelude::Context;

pub trait Contextual<T> {
  fn resolve(&self, ctx: &Context) -> T;
}
