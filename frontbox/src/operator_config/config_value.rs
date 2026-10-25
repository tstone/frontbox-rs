use std::marker::PhantomData;

use crate::operator_config::{
  ConfigDisplay, ConfigType, IntoConfigDomain, OperatorConfig, RawConfigValue,
};
use crate::prelude::SystemContext;

/// A config value declaration: its [`ConfigDisplay`] plus the Rust type it's read as
#[derive(Clone, Debug)]
pub struct ConfigValue<T> {
  display: ConfigDisplay,
  _type: PhantomData<fn() -> T>,
}

impl<T: ConfigType> ConfigValue<T> {
  pub fn new(
    name: &'static str,
    desc: &'static str,
    default: T,
    domain: impl IntoConfigDomain<T>,
  ) -> Self {
    Self {
      display: ConfigDisplay {
        name,
        desc,
        domain: domain.into_domain(),
        default: default.to_raw(),
        formatter: format_raw::<T>,
      },
      _type: PhantomData,
    }
  }

  /// The untyped declaration, for listing alongside configs of other types
  pub fn display(&self) -> &ConfigDisplay {
    &self.display
  }

  pub fn default(&self) -> T {
    T::from_raw(self.display.default)
  }

  pub fn get(&self, ctx: &SystemContext) -> T {
    ctx.operator_config.get(self)
  }

  pub fn resolve(&self, op: &OperatorConfig) -> T {
    op.get(self)
  }
}

fn format_raw<T: ConfigType>(raw: RawConfigValue) -> String {
  T::from_raw(raw).display()
}
