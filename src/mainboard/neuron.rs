use std::thread;

use crate::mainboard::mainboard::Mainboard;
use crate::protocol::FastResponse;
use crate::protocol::id;
use crate::serial::serial_interface::SerialInterface;

pub struct Neuron {
  config: NeuronConfig,
  io_net_port: Option<SerialInterface>,
  exp_port: Option<SerialInterface>,
}

pub struct NeuronConfig {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
}

impl Neuron {
  pub fn define(config: NeuronConfig) -> Self {
    Self {
      config,
      io_net_port: None,
      exp_port: None,
    }
  }
}

impl Mainboard for Neuron {
  fn initialize(&mut self) {
    let mut io_net_port = SerialInterface::open(self.config.io_net_port_path);

    // Wait for boot (ID response)
    let mut resp: Option<FastResponse> = None;
    io_net_port.send(id::request());
    while resp.is_none() || matches!(resp, Some(FastResponse::Failed(_))) {
      resp = io_net_port.read_next();
      log::trace!("I/O Net boot response: {:?}", resp);
      thread::sleep(std::time::Duration::from_millis(10));
    }
    self.io_net_port = Some(io_net_port);

    self.exp_port = Some(SerialInterface::open(self.config.exp_port_path));

    // TODO: Watchdog
    // TODO: game loop or async?
  }
}
