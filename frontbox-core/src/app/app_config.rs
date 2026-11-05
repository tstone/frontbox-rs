use std::time::Duration;

use crate::app::BootConfig;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
  pub system_interval: Duration,
  pub watchdog_interval: Option<Duration>,
}

impl AppConfig {
  pub fn from_boot_config(boot: &BootConfig) -> Self {
    Self {
      system_interval: boot.system_interval,
      watchdog_interval: boot.platform.watchdog_interval(),
    }
  }
}
