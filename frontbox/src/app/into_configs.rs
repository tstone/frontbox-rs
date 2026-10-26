use crate::{operator_config::ConfigDefinition, systems::System};

pub trait IntoConfigs {
  fn into_configs(self) -> Vec<&'static ConfigDefinition>;
}

impl<T: System + 'static> IntoConfigs for &T {
  fn into_configs(self) -> Vec<&'static ConfigDefinition> {
    self.configs()
  }
}

impl IntoConfigs for Vec<&'static ConfigDefinition> {
  fn into_configs(self) -> Vec<&'static ConfigDefinition> {
    self
  }
}
