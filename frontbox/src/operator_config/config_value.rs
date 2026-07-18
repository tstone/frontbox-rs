use std::fmt::Debug;

use crate::operator_config::{Domain, OperatorConfig};
use crate::prelude::Context;

#[derive(Debug, Clone)]
pub struct ConfigValue<T, D: Domain<T>> {
  pub name: &'static str,
  pub desc: &'static str,
  pub default: T,
  pub domain: D,
}

impl<T, D: Domain<T>> ConfigValue<T, D>
where
  T: Clone + Send + Sync + 'static,
{
  pub fn get(&self, ctx: &Context) -> T {
    ctx.operator_config.get(self)
  }

  pub fn resolve(&self, op: &OperatorConfig) -> T {
    op.get(self)
  }
}

impl<T, D: Domain<T>> ConfigValue<T, D> {
  pub fn new(name: &'static str, desc: &'static str, default: T, domain: D) -> Self {
    ConfigValue {
      name,
      desc,
      default,
      domain,
    }
  }
}
