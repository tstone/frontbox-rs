use std::path::Path;

use crate::{ExpNetwork, IoNetwork};

pub struct BootConfig {
  // TODO: is it possible to autodetect ports by looking at the USB device info?
  /// Port path for the I/O network. Will be something like `/dev/ttyACM0` on unix/linux/mac and something like "COM3" on Windows
  pub io_net_port_path: &'static str,
  /// Port path for the EXP network. Will be something like `/dev/ttyACM0` on unix/linux/mac and something like "COM3" on Windows
  pub exp_port_path: &'static str,
  pub io_network: IoNetwork,
  pub exp_network: ExpNetwork,
  /// Disk path for the operator config TOML file. Process must have read/write access to this file.
  pub config_path: Option<&'static Path>,
}

impl Default for BootConfig {
  fn default() -> Self {
    BootConfig {
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      io_network: IoNetwork::empty(),
      exp_network: ExpNetwork::empty(),
      config_path: None,
    }
  }
}