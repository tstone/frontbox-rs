mod fast_codec;
mod io_network;
pub mod serial_interface;

use std::time::Duration;

pub use fast_codec::*;
pub use io_network::*;

#[derive(Debug, Clone)]
pub struct BootConfig {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub platform: FastPlatform,
  pub watchdog_interval: Duration,
}

impl Default for BootConfig {
  fn default() -> Self {
    Self {
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      platform: FastPlatform::Neuron,
      watchdog_interval: Duration::from_millis(1250),
    }
  }
}

#[derive(Debug, Clone)]
pub enum FastPlatform {
  Neuron = 2000,
  RetroSystem11 = 11,
  RetroWPC89 = 89,
  RetroWPC95 = 95,
}
