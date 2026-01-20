use bevy_app::{ScheduleRunnerPlugin, prelude::*};
use bevy_ecs::prelude::*;
use frontbox::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() {
  env_logger::init();

  App::new()
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
    .add_plugins(Frontbox {
      mainboard_config: MainboardConfig {
        io_net_port_path: "/dev/ttyACM0",
        exp_port_path: "/dev/ttyACM1",
        platform: FastPlatform::Neuron,
        switch_reporting: None,
      },
    })
    .add_systems(Startup, startup)
    .run();
}

fn startup(mut mainboard: ResMut<Mainboard>) {
  log::info!("App started with mainboard: {:?}", mainboard);
  mainboard.enable_watchdog();
}
