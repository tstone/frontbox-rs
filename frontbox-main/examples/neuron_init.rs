extern crate frontbox_main;

use frontbox_main::{FastPlatform, Mainboard, MainboardConfig};

#[tokio::main]
async fn main() {
  env_logger::init();

  let mut neuron = Mainboard::new(MainboardConfig {
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
    platform: FastPlatform::Neuron,
    ..Default::default()
  });

  neuron.run().await;
}
