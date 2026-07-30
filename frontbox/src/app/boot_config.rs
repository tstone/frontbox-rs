use std::path::Path;

use crate::{ExpNetwork, IoNetwork};

pub struct BootConfig {
  // TODO: is it possible to just autodetect this?
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub io_network: IoNetwork,
  pub exp_network: ExpNetwork,
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