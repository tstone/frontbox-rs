use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::operator_config::generalized_config_value::GeneralizedConfigValue;
use crate::operator_config::*;

/// A value that can be assigned to configure a piece of hardware, either with a fixed value or using operator config
#[derive(Clone, Debug)]
pub enum HardwareValue<T: Clone, D: Domain<T> = Range<T>> {
  Config(ConfigValue<T, D>),
  Fixed(T),
}

impl<T, D: Domain<T>> HardwareValue<T, D>
where
  T: Clone + Send + Sync + 'static,
{
  pub fn fixed(value: T) -> Self {
    Self::Fixed(value)
  }

  pub fn config(name: &'static str, desc: &'static str, default: T, domain: D) -> Self {
    Self::Config(ConfigValue {
      name,
      desc,
      default,
      domain,
    })
  }

  /// Get the actual usable value given a working environment
  pub fn resolve(&self, op: &OperatorConfig) -> T {
    match self {
      Self::Fixed(v) => v.clone(),
      Self::Config(cv) => cv.resolve(op),
    }
  }

  /// Typed version for TOML read/write
  pub fn generalized_config_value(&self) -> Option<&dyn GeneralizedConfigValue>
  where
    T: ConfigDisplay + Serialize + DeserializeOwned + Send + Sync,
    D: Send + Sync,
  {
    match self {
      Self::Config(cv) => Some(cv),
      Self::Fixed(_) => None,
    }
  }
}
