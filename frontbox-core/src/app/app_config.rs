use std::path::PathBuf;
use std::time::Duration;

use crate::app::{BootConfig, DEFAULT_LED_RADIUS};
use crate::location::AppPlane;

#[derive(Debug, Clone)]
pub struct AppConfig {
  pub system_interval: Duration,
  pub watchdog_interval: Option<Duration>,
  pub led_radius: f32,
  /// See [`BootConfig::content_path`]
  pub content_path: Option<PathBuf>,
  /// See [`App::plane`](crate::app::App::plane)
  pub planes: Vec<AppPlane>,
}

impl AppConfig {
  pub fn from_boot_config(boot: &BootConfig) -> Self {
    Self {
      system_interval: boot.system_interval,
      watchdog_interval: boot.platform.watchdog_interval(),
      led_radius: boot.led_radius,
      content_path: boot.content_path.clone(),
      planes: Vec::new(),
    }
  }
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      system_interval: Duration::ZERO,
      watchdog_interval: None,
      led_radius: DEFAULT_LED_RADIUS,
      content_path: None,
      planes: Vec::new(),
    }
  }
}
