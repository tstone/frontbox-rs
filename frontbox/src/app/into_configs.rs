use crate::{operator_config::ConfigDisplay, systems::System};

pub trait IntoConfigs {
  fn into_configs(self) -> Vec<&'static ConfigDisplay>;
}

impl<T: System + 'static> IntoConfigs for &T {
  fn into_configs(self) -> Vec<&'static ConfigDisplay> {
    self.configs()
  }
}

impl IntoConfigs for Vec<&'static ConfigDisplay> {
  fn into_configs(self) -> Vec<&'static ConfigDisplay> {
    self
  }
}
