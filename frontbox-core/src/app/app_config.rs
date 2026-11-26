use std::sync::Arc;
use std::time::Duration;

use crate::app::{BootConfig, DEFAULT_LIGHT_RADIUS, PlatformKind, Resources};
use crate::location::AppPlane;

#[derive(Debug, Clone)]
pub struct AppConfig {
  /// See [`Platform::kind`](crate::app::Platform::kind)
  pub platform: PlatformKind,
  pub system_interval: Duration,
  pub watchdog_interval: Option<Duration>,
  pub light_radius: f32,
  /// What's in the content folder, and what code added while the app was built. See [`App::resources`](crate::app::App::resources)
  pub resources: Arc<Resources>,
  /// See [`App::plane`](crate::app::App::plane)
  pub planes: Vec<AppPlane>,
}

impl AppConfig {
  /// With only what's in the content folder as its resources
  pub fn from_boot_config(boot: &BootConfig) -> Self {
    Self {
      platform: boot.platform.kind(),
      system_interval: boot.system_interval,
      watchdog_interval: boot.platform.watchdog_interval(),
      light_radius: boot.light_radius,
      resources: Arc::new(Resources::in_folder(boot.content_path.clone())),
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
      resources: Arc::default(),
      planes: Vec::new(),
    }
  }
}
