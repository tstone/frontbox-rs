use std::path::PathBuf;
use std::time::Duration;

use crate::app::{BootConfig, DEFAULT_LIGHT_RADIUS, PlatformKind};
use crate::location::AppPlane;

#[derive(Debug, Clone)]
pub struct AppConfig {
  /// See [`Platform::kind`](crate::app::Platform::kind)
  pub platform: PlatformKind,
  pub system_interval: Duration,
  pub watchdog_interval: Option<Duration>,
  pub light_radius: f32,
  /// See [`BootConfig::content_path`]
  pub content_path: Option<PathBuf>,
  /// See [`App::plane`](crate::app::App::plane)
  pub planes: Vec<AppPlane>,
}

impl AppConfig {
  pub fn from_boot_config(boot: &BootConfig) -> Self {
    Self {
      platform: boot.platform.kind(),
      system_interval: boot.system_interval,
      watchdog_interval: boot.platform.watchdog_interval(),
      light_radius: boot.light_radius,
      content_path: boot.content_path.clone(),
      planes: Vec::new(),
    }
  }
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      platform: PlatformKind::Virtual,
      system_interval: Duration::ZERO,
      watchdog_interval: None,
      light_radius: DEFAULT_LIGHT_RADIUS,
      content_path: None,
      planes: Vec::new(),
    }
  }
}
