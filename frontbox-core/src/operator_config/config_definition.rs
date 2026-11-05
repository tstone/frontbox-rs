use crate::operator_config::{ConfigDomain, ConfigDomainValue};

#[derive(Clone, Debug)]
pub struct ConfigDefinition {
  /// Identity: where the value is stored and saved. The label unless set with [`Config::key`](crate::operator_config::Config::key).
  pub key: &'static str,
  /// What the operator sees
  pub label: &'static str,
  pub desc: &'static str,
  /// The Rust type the value is read as, so two types stored as the same domain value don't share a key
  pub type_name: &'static str,
  pub domain: ConfigDomain,
  pub default: ConfigDomainValue,
  pub(crate) formatter: fn(ConfigDomainValue) -> String,
}

impl ConfigDefinition {
  /// How a value is shown to the operator
  pub fn format(&self, value: ConfigDomainValue) -> String {
    (self.formatter)(value)
  }

  pub fn is_default(&self, value: ConfigDomainValue) -> bool {
    value == self.default
  }

  /// Whether a value stored for one definition reads correctly through the other. Label and description may differ.
  pub fn is_compatible(&self, other: &ConfigDefinition) -> bool {
    self.type_name == other.type_name
      && self.domain == other.domain
      && self.default == other.default
  }
}
