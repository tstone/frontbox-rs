use std::marker::PhantomData;

use crate::operator_config::{ConfigDefinition, ConfigDomainValue, ConfigType, IntoConfigDomain};
use crate::prelude::SystemContext;

/// A typed config value. Joins the definition of a config with it's actual type.
#[derive(Clone, Debug)]
pub struct Config<T> {
  display: ConfigDefinition,
  _type: PhantomData<fn() -> T>,
}

impl<T: ConfigType> Config<T> {
  pub fn new(
    label: &'static str,
    desc: &'static str,
    default: T,
    domain: impl IntoConfigDomain<T>,
  ) -> Self {
    Self {
      display: ConfigDefinition {
        key: label,
        label,
        desc,
        type_name: std::any::type_name::<T>(),
        domain: domain.into_domain(),
        default: default.to_value(),
        formatter: format_value::<T>,
      },
      _type: PhantomData,
    }
  }

  /// Set the config's `key`. Key must be unique but is derived from label by default. This force-sets it to a specific value for uniqueness.
  pub fn key(mut self, key: &'static str) -> Self {
    self.display.key = key;
    self
  }

  /// The untyped declaration, for listing alongside configs of other types
  pub fn display(&self) -> &ConfigDefinition {
    &self.display
  }

  pub fn default(&self) -> T {
    T::from_value(self.display.default)
  }

  pub fn get(&self, ctx: &SystemContext) -> T {
    ctx.config.get(self)
  }
}

fn format_value<T: ConfigType>(value: ConfigDomainValue) -> String {
  T::from_value(value).display()
}
