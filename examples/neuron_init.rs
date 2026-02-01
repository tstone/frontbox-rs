use frontbox::prelude::*;
use std::default;
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

  // boot mainboard
  let mut machine = Machine::boot(
    BootConfig {
      platform: FastPlatform::Neuron,
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      ..Default::default()
    },
    io_network.build(),
  )
  .await;

  machine
    .add_machine_frame(vec![Freeplay::new(Switches::START_BUTTON, 4)])
    .run()
    .await;
}
