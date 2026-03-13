use core::panic;
use std::collections::HashMap;

use crate::DriverMode;
use crate::hardware_definition::io::SwitchConfig;

#[derive(Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u32,
  pub(crate) driver_count: u32,
  pub(crate) switch_map: HashMap<u16, &'static str>,
  pub(crate) driver_map: HashMap<u16, &'static str>,
  pub(crate) switch_configs: HashMap<&'static str, SwitchConfig>,
  pub(crate) driver_configs: HashMap<&'static str, Box<dyn DriverMode>>,
}

impl IoBoardBuilder {
  pub fn with_switch(mut self, name: &'static str, pin: u16) -> Self {
    if pin >= self.switch_count as u16 {
      panic!(
        "Switch index {} out of bounds for board with {} switches",
        pin, self.switch_count
      );
    }

    self.switch_map.insert(pin, name);
    self
  }

  pub fn with_switch_cfg(mut self, name: &'static str, pin: u16, config: SwitchConfig) -> Self {
    self = self.with_switch(name, pin);
    self.switch_configs.insert(name, config);
    self
  }

  pub fn with_driver_cfg(
    mut self,
    name: &'static str,
    pin: u16,
    config: impl DriverMode + 'static,
  ) -> Self {
    if pin >= self.driver_count as u16 {
      panic!(
        "Driver index {} out of bounds for board with {} drivers",
        pin, self.driver_count
      );
    }

    self.driver_map.insert(pin, name);
    self.driver_configs.insert(name, Box::new(config));
    self
  }
}
