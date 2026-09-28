use std::ops::Deref;

use crate::operator_config::OperatorConfig;
use crate::prelude::*;

#[derive(Default)]
pub struct BootSnapshot {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
  pub operator_config: OperatorConfig,
  pub(crate) app_config: AppConfig,
}

impl BootSnapshot {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    leds: LedLookup,
    io_network: Vec<ResolvedIoBoard>,
    exp_network: Vec<ResolvedExpansionBoard>,
    operator_config: OperatorConfig,
    app_config: AppConfig,
  ) -> Self {
    Self {
      switches,
      drivers,
      leds,
      io_network,
      exp_network,
      operator_config,
      app_config,
    }
  }

  pub fn from_hardware(
    hardware: Hardware,
    operator_config: OperatorConfig,
    app_config: AppConfig,
  ) -> Self {
    Self {
      switches: hardware.switches,
      drivers: hardware.drivers,
      leds: hardware.leds,
      io_network: hardware.io_network,
      exp_network: hardware.exp_network,
      app_config,
      operator_config,
    }
  }
}

impl Deref for BootSnapshot {
  type Target = AppConfig;

  fn deref(&self) -> &Self::Target {
    &self.app_config
  }
}
