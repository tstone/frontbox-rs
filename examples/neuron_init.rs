use bevy_app::{ScheduleRunnerPlugin, prelude::*};
use bevy_ecs::prelude::*;
use frontbox::mainboard_io::MainboardIncoming;
use frontbox::prelude::*;
use frontbox::protocol::configure_hardware::SwitchReporting;
use std::time::Duration;

pub mod Switches {
  pub const START_BUTTON: &str = "start_button";
  pub const LEFT_FLIPPER_BUTTON: &str = "left_flipper_button";
  pub const RIGHT_FLIPPER_BUTTON: &str = "right_flipper_button";
  pub const LEFT_OUTLANE: &str = "left_outlane";
  pub const LEFT_INLANE: &str = "left_inlane";
}

pub mod Drivers {
  pub const START_BUTTON_LAMP: &str = "start_button_lamp";
  pub const LEFT_FLIPPER_MAIN_COIL: &str = "left_flipper_main_coil";
  pub const LEFT_FLIPPER_HOLD_COIL: &str = "left_flipper_hold_coil";
  pub const RIGHT_FLIPPER_MAIN_COIL: &str = "right_flipper_main_coil";
  pub const RIGHT_FLIPPER_HOLD_COIL: &str = "right_flipper_hold_coil";
}

#[tokio::main]
async fn main() {
  env_logger::init();

  // boot mainboard
  let mainboard = Mainboard::boot(BootConfig {
    platform: FastPlatform::Neuron,
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
  })
  .await;

  let mut io_network = IoNetworkSpec::new();

  io_network.add_board(
    FastIoBoards::cabinet()
      .with_switch(0, Switches::START_BUTTON)
      .with_driver_pin(0, Drivers::START_BUTTON_LAMP),
  );

  io_network.add_board(
    FastIoBoards::io_3208()
      .with_driver_pin(0, Drivers::LEFT_FLIPPER_MAIN_COIL)
      .with_driver_pin(1, Drivers::LEFT_FLIPPER_HOLD_COIL),
  );

  // run engine
  App::new()
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
    .add_plugins(Frontbox {
      mainboard,
      io_network: io_network.build(),
    })
    .add_systems(Startup, startup)
    .add_observer(on_mainboard_event)
    .run();
}

fn startup(mut mainboard: ResMut<MainboardLink>) {
  log::info!("😀 Neuron init example started");
  mainboard.enable_watchdog();
}

// example of listening to raw events from the Neuron
fn on_mainboard_event(event: On<MainboardIncoming>) {
  log::info!("📧 Received mainboard event: {:?}", event);
}
