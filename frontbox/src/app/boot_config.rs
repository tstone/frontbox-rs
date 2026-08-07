use std::{path::{PathBuf}, time::Duration};

use crate::hardware::{ExpNetwork, IoNetwork};

/// Immutable values used by App to run everything
/// 
/// # Examples
/// 
/// ```rust,no_run
/// let app = App::boot(BootConfig {
///   io_net_port_path: "/dev/ttyACM0",
///   io_network: IoNetwork::new(vec![ /* boards defined here */ ]),
///   system_interval: Duration::from_millis(83),
///   config_path: PathBuf::from("/game/operator_config.toml"),
///   ..Default::default()
/// })
/// .await;
/// ```
pub struct BootConfig {
  // TODO: is it possible to autodetect ports by looking at the USB device info?
  /// Port path for the I/O network. Will be something like `/dev/ttyACM0` on unix/linux/mac or something like "COM3" on Windows
  pub io_net_port_path: &'static str,
  /// Port path for the EXP network. Will be something like `/dev/ttyACM0` on unix/linux/mac or something like "COM5" on Windows
  pub exp_port_path: &'static str,
  /// Definition of I/O network hardware
  pub io_network: IoNetwork,
  /// Definition of expansion network hardware
  pub exp_network: ExpNetwork,
  /// File for operator config TOML. Any previously edited values will be loaded from here, and any chnages made (e.g. via the operator menu) will be saved here. Process must have read+write access.
  pub config_path: Option<PathBuf>,
  /// The interval at which `tick` and `render` run. This affects both the resolution of timers and LED + display render speed.
  /// Higher values result in higher resolution and smoother animation but additional CPU load. If this value is set too high
  /// it will introduce latency in responding to events.
  /// 
  /// Defaults to 83ms / 12Hz / 12 FPS, which is probably overly conservative for most hardware
  pub system_interval: Duration,
  /// FAST hardware requires the software to "ping" it every so often as a safety mechanism, signalling that the software is
  /// still running and in control. This is the interval at which that happens (watchdog). This should be set high enough not
  /// to become a burden to the system, but low enough that the hardware can safely shut down if it fails. Think of this value
  /// as the longest you would want a coil to be at full power if the software were to crash.
  /// 
  /// Defaults to 1.5s
  pub watchdog_interval: Duration,
}

impl Default for BootConfig {
  fn default() -> Self {
    BootConfig {
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      io_network: IoNetwork::empty(),
      exp_network: ExpNetwork::empty(),
      config_path: None,
      system_interval: Duration::from_millis(83), // 12 fps
      watchdog_interval: Duration::from_millis(1500),
    }
  }
}