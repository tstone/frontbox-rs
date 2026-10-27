use crate::operator_config::{ConfigDomain, ConfigType, IntoConfigDomain};

/// A config that is one of several named choices. `T` maps itself to and from the choice's index.
pub struct Choices(pub &'static [&'static str]);

impl<T: ConfigType> IntoConfigDomain<T> for Choices {
  fn into_domain(self) -> ConfigDomain {
    ConfigDomain::Choice(self.0)
  }
}

/// A config that's either on or off
pub struct Toggle;

impl IntoConfigDomain<bool> for Toggle {
  fn into_domain(self) -> ConfigDomain {
    ConfigDomain::Bool
  }
}
