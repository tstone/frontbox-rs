use core::panic;
use std::collections::HashMap;

use crate::hardware_definition::io::SwitchConfig;
use fast_protocol::DriverConfig;

#[derive(Debug, Clone, Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u32,
  pub(crate) driver_count: u32,
  pub(crate) switch_map: HashMap<u16, &'static str>,
  pub(crate) driver_map: HashMap<u16, &'static str>,
  pub(crate) switch_configs: HashMap<&'static str, SwitchConfig>,
  pub(crate) driver_configs: HashMap<&'static str, DriverConfig>,
}

impl IoBoardBuilder {
  pub fn with_switch(mut self, idx: u16, key: &'static str) -> Self {
    if idx >= self.switch_count as u16 {
      panic!(
        "Switch index {} out of bounds for board with {} switches",
        idx, self.switch_count
      );
    }

    self.switch_map.insert(idx, key);
    self
  }

  pub fn with_switch_cfg(mut self, idx: u16, key: &'static str, config: SwitchConfig) -> Self {
    self = self.with_switch(idx, key);
    self.switch_configs.insert(key, config);
    self
  }

  pub fn with_driver_cfg<T: Into<DriverConfig>>(
    mut self,
    idx: u16,
    key: &'static str,
    config: T,
  ) -> Self {
    if idx >= self.driver_count as u16 {
      panic!(
        "Driver index {} out of bounds for board with {} drivers",
        idx, self.driver_count
      );
    }

    self.driver_map.insert(idx, key);
    self.driver_configs.insert(key, config.into());
    self
  }
}
