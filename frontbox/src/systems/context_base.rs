use std::ops::Deref;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ContextBase {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub io_network: Vec<IoBoard>,
  pub exp_network: Vec<ExpansionBoard>,
  pub(crate) app_config: AppConfig,
}

impl Deref for ContextBase {
  type Target = AppConfig;

  fn deref(&self) -> &Self::Target {
    &self.app_config
  }
}
