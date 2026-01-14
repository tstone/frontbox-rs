extern crate frontbox_main;

use frontbox_main::{FastPlatform, Mainboard, MainboardConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
  env_logger::init();

  let neuron = Mainboard::new(MainboardConfig {
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
    platform: FastPlatform::Neuron,
    ..Default::default()
  });

  let neuron_ref = Arc::new(Mutex::new(neuron));

  neuron_ref.lock().await.enable_watchdog();
  neuron_ref.lock().await.run().await;
}
