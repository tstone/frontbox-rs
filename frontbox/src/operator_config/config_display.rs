use crate::operator_config::{ConfigDomain, RawConfigValue};

/// Everything about a config value except its Rust type, so configs of different types can be listed together (for
/// registration, menus, persistence). Built by [`ConfigValue::new`](crate::operator_config::ConfigValue::new).
#[derive(Clone, Debug)]
pub struct ConfigDisplay {
  pub name: &'static str,
  pub desc: &'static str,
  pub domain: ConfigDomain,
  pub default: RawConfigValue,
  pub(crate) formatter: fn(RawConfigValue) -> String,
}

impl ConfigDisplay {
  /// How a value is shown to the operator
  pub fn format(&self, value: RawConfigValue) -> String {
    (self.formatter)(value)
  }

  pub fn is_default(&self, value: RawConfigValue) -> bool {
    value == self.default
  }
}
