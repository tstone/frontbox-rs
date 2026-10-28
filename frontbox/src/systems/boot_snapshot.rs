use std::ops::Deref;

use crate::operator_config::ConfigCollection;
use crate::prelude::*;

#[derive(Default)]
pub struct BootSnapshot {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
  pub config: ConfigCollection,
  pub(crate) app_config: AppConfig,
}

impl BootSnapshot {
  pub fn from_hardware(
    hardware: Hardware,
    config: ConfigCollection,
    app_config: AppConfig,
  ) -> Self {
    Self {
      switches: hardware.switches,
      drivers: hardware.drivers,
      leds: hardware.leds,
      io_network: hardware.io_network,
      exp_network: hardware.exp_network,
      app_config,
      config,
    }
  }
}

impl Deref for BootSnapshot {
  type Target = AppConfig;

  fn deref(&self) -> &Self::Target {
    &self.app_config
  }
}
