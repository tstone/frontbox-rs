use std::{path::PathBuf, time::Duration};

use crate::hardware::{ExpNetwork, IoNetwork};

/// Half the width of a common 5050 addressable LED (5mm square), in inches
pub const DEFAULT_LED_RADIUS: f32 = 0.1;

/// Immutable values used by App to run everything
///
/// # Examples
///
/// ```rust,no_run
/// # use frontbox_core::prelude::*;
/// # use std::path::PathBuf;
/// let app = App::new(BootConfig {
///   platform: Platform::Neuron {
///     io_net_port_path: "/dev/ttyACM0",
///     exp_port_path: "/dev/ttyACM1",
///     watchdog_interval: Duration::from_millis(1500),
///   },
///   io_network: IoNetwork::new(vec![ /* boards defined here */ ]),
///   system_interval: Duration::from_millis(83),
///   config_path: Some(PathBuf::from("/game/operator_config.toml")),
///   ..Default::default()
/// });
/// ```
pub struct BootConfig {
  pub platform: Platform,
  /// Definition of I/O network hardware
  pub io_network: IoNetwork,
  /// Definition of expansion network hardware
  pub exp_network: ExpNetwork,
  /// File for operator config TOML. Any previously edited values will be loaded from here, and any changes made (e.g. via the operator menu) will be saved here. Process must have read+write access.
  ///
  /// Defaults to `operator_config.toml` in the working directory. `None` keeps changes in memory only.
  pub config_path: Option<PathBuf>,
  /// Directory for the metrics database. Recorded metrics are flushed here at the end of each game, every 30 seconds,
  /// and at shutdown. Process must have read+write access.
  ///
  /// Defaults to `metrics.db` in the working directory. `None` counts metrics in memory and drops them.
  pub metrics_path: Option<PathBuf>,
  /// Folder of what scenes use, such as images. It's scanned whatever is inside it, so organize it any way you like.
  /// Scenes name images by their path from here, so the folder can move without changing them. Process must have read
  /// access, and write access for the console to add to it.
  ///
  /// A relative path is resolved from the working directory, which depends on how the game is started: `cargo run` uses
  /// wherever it was run from and a service uses whatever it was configured with. `env!("CARGO_MANIFEST_DIR")` is the
  /// folder the game was built in, which is only there on the machine that built it. For a game that's built and run in
  /// different ways, give this an absolute path chosen at startup, as with the other paths here. Debug and release builds
  /// run from different places, so an absolute path is also how they share a folder. See the README for an example.
  ///
  /// Defaults to `content` in the working directory. `None` means there isn't one, so a scene can't draw images.
  pub content_path: Option<PathBuf>,
  /// The interval at which `tick` and `render` run. This affects both the resolution of timers and LED + display render speed.
  /// Higher values result in higher resolution and smoother animation but additional CPU load. If this value is set too high
  /// it will introduce latency in responding to events.
  ///
  /// Defaults to 83ms / 12Hz / 12 FPS, which is probably overly conservative for most hardware
  pub system_interval: Duration,
  /// The radius of the area an LED lights, in the units of its location. This is how far a scene's shape edges fade
  /// across an LED.
  ///
  /// Defaults to [`DEFAULT_LED_RADIUS`]
  pub led_radius: f32,
}

impl Default for BootConfig {
  fn default() -> Self {
    BootConfig {
      io_network: IoNetwork::empty(),
      exp_network: ExpNetwork::empty(),
      config_path: Some(PathBuf::from("operator_config.toml")),
      metrics_path: Some(PathBuf::from("metrics.db")),
      content_path: Some(PathBuf::from("content")),
      system_interval: Duration::from_millis(83), // 12 fps
      led_radius: DEFAULT_LED_RADIUS,
      // watchdog_interval: Duration::from_millis(1500),
      platform: Platform::Virtual,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
  Neuron {
    /// Port path for the I/O network. Will be something like `/dev/ttyACM0` on unix/linux/mac or something like "COM3" on Windows
    io_net_port_path: &'static str,
    /// Port path for the EXP network. Will be something like `/dev/ttyACM0` on unix/linux/mac or something like "COM5" on Windows
    exp_port_path: &'static str,
    /// FAST hardware requires the software to "ping" it every so often as a safety mechanism, signalling that the software is
    /// still running and in control. This is the interval at which that happens (watchdog). This should be set high enough not
    /// to become a burden to the system, but low enough that the hardware can safely shut down if it fails. Think of this value
    /// as the longest you would want a coil to be at full power if the software were to crash.
    ///
    /// Defaults to 1.5s
    watchdog_interval: Duration,
  },
  Virtual,
}

impl Platform {
  pub fn watchdog_interval(&self) -> Option<Duration> {
    match self {
      Platform::Neuron {
        watchdog_interval, ..
      } => Some(*watchdog_interval),
      _ => None,
    }
  }
}
