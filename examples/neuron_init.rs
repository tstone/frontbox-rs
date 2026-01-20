use bevy_app::{ScheduleRunnerPlugin, prelude::*};
use bevy_ecs::prelude::*;
use frontbox::mainboard_comms::MainboardIncoming;
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
    .add_observer(on_mainboard_event)
    .run();
}

fn startup(mut mainboard: ResMut<Mainboard>) {
  log::info!("😀 Neuron init example started");
  mainboard.enable_watchdog();
}

// example of listening to raw events from the Neuron
fn on_mainboard_event(event: On<MainboardIncoming>) {
  log::info!("📧 Received mainboard event: {:?}", event);
}
