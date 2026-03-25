use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ContextBase {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub io_network: Vec<IoBoard>,
  pub exp_network: Vec<ExpansionBoard>,
  pub app_config: AppConfig,
}
