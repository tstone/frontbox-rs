use crate::operator_config::{ConfigDomain, ConfigDomainValue};

#[derive(Clone, Debug)]
pub struct ConfigDefinition {
  pub name: &'static str,
  pub desc: &'static str,
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
}
