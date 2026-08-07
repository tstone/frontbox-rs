use crate::{operator_config::GeneralizedConfigValue, systems::System};

pub trait IntoConfigs {
  fn into_configs(self) -> Vec<&'static dyn GeneralizedConfigValue>;
}

impl<T: System + 'static> IntoConfigs for &T {
  fn into_configs(self) -> Vec<&'static dyn GeneralizedConfigValue> {
    self.config_values()
  }
}

impl IntoConfigs for Vec<&'static dyn GeneralizedConfigValue> {
  fn into_configs(self) -> Vec<&'static dyn GeneralizedConfigValue> {
    self
  }
}