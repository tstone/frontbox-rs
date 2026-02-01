use frontbox::prelude::*;
use std::io::Write;

pub mod switches {
  pub const START_BUTTON: &str = "start_button";
  pub const LEFT_FLIPPER_BUTTON: &str = "left_flipper_button";
  pub const RIGHT_FLIPPER_BUTTON: &str = "right_flipper_button";
  pub const LEFT_OUTLANE: &str = "left_outlane";
  pub const LEFT_INLANE: &str = "left_inlane";
}

pub mod drivers {
  pub const START_BUTTON_LAMP: &str = "start_button_lamp";
  pub const LEFT_FLIPPER_MAIN_COIL: &str = "left_flipper_main_coil";
  pub const LEFT_FLIPPER_HOLD_COIL: &str = "left_flipper_hold_coil";
  pub const RIGHT_FLIPPER_MAIN_COIL: &str = "right_flipper_main_coil";
  pub const RIGHT_FLIPPER_HOLD_COIL: &str = "right_flipper_hold_coil";
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let mut io_network = IoNetworkSpec::new();

  io_network.add_board(
    FastIoBoards::cabinet()
      .with_switch(0, switches::START_BUTTON)
      // .with_switch_config(0, SwitchConfig { debounce_ms: 50, invert: false })
      .with_driver_pin(0, drivers::START_BUTTON_LAMP),
  );

  io_network.add_board(
    FastIoBoards::io_3208()
      .with_driver_pin(0, drivers::LEFT_FLIPPER_MAIN_COIL)
      .with_driver_pin(1, drivers::LEFT_FLIPPER_HOLD_COIL),
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
    .add_keyboard_mapping(KeyCode::Home, switches::START_BUTTON)
    .add_machine_frame(vec![Freeplay::new(switches::START_BUTTON, 4)])
    .run()
    .await;
}
