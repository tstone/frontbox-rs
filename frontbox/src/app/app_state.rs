use std::ops::Deref;

use crate::metrics::MetricsCollection;
use crate::operator_config::ConfigCollection;
use crate::prelude::*;

#[derive(Default)]
pub struct AppState {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
  pub config: ConfigCollection,
  pub metrics: MetricsCollection,
  pub(crate) app_config: AppConfig,
}

impl AppState {
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
      metrics: MetricsCollection::default(),
    }
  }
}

impl Deref for AppState {
  type Target = AppConfig;

  fn deref(&self) -> &Self::Target {
    &self.app_config
  }
}

#[cfg(test)]
mod tests {
  use crate::prelude::*;

  #[test]
  fn metrics_are_recorded_through_context() {
    let saves = Counter::new("saves", "");
    let test_ctx = TestContext::default();

    let ctx = test_ctx.sys_ctx();
    ctx.metrics.inc(&saves);

    assert_eq!(test_ctx.base.metrics.take_batch().counts[0].count, 1);
  }
}
