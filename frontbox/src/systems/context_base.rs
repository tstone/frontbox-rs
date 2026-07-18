use std::ops::Deref;

use crate::operator_config::OperatorConfig;
use crate::prelude::*;

// #[derive(Clone)]
pub struct ContextBase {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
  pub operator_config: OperatorConfig,
  pub(crate) app_config: AppConfig,
}

impl Deref for ContextBase {
  type Target = AppConfig;

  fn deref(&self) -> &Self::Target {
    &self.app_config
  }
}
