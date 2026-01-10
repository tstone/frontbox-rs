extern crate tokio_pinball;

use tokio_pinball::{FastPlatform, Mainboard, MainboardConfig};

#[tokio::main]
async fn main() {
  env_logger::init();

  let mut neuron = Mainboard::new(MainboardConfig {
    io_net_port_path: "/dev/ttyACM0",
    platform: FastPlatform::Neuron,
    ..Default::default()
  });

  neuron.run().await;
}
