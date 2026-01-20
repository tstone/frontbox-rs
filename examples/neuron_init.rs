use bevy_app::{App, ScheduleRunnerPlugin};
use frontbox::{FastPlatform, MainboardPlugin};
use std::time::Duration;

#[tokio::main]
async fn main() {
  env_logger::init();

  App::new()
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
    .add_plugins(MainboardPlugin {
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      platform: FastPlatform::Neuron,
      switch_reporting: None,
    })
    .run();
}
