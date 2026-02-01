use frontbox::prelude::*;
use std::io::Write;

pub mod switches {
  pub const START_BUTTON: &str = "start_button";
  pub const LOWER_DROP_TARGET1: &str = "lower_drop_target1";
  pub const LOWER_DROP_TARGET2: &str = "lower_drop_target2";
  pub const LOWER_DROP_TARGET3: &str = "lower_drop_target3";
}

pub mod drivers {
  pub const START_BUTTON_LAMP: &str = "start_button_lamp";
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
      // TODO: .with_switch_config(0, SwitchConfig { debounce_ms: 50, invert: false })
      .with_driver_pin(0, drivers::START_BUTTON_LAMP),
  );

  io_network.add_board(
    FastIoBoards::io_3208()
      .with_switch(0, switches::LOWER_DROP_TARGET1)
      .with_switch(1, switches::LOWER_DROP_TARGET2)
      .with_switch(2, switches::LOWER_DROP_TARGET3),
  );

  Machine::boot(
    BootConfig {
      platform: FastPlatform::Neuron,
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      ..Default::default()
    },
    io_network.build(),
  )
  .await
  .add_keyboard_mapping(KeyCode::Home, switches::START_BUTTON)
  .add_machine_frame(vec![Freeplay::new(switches::START_BUTTON, 4)])
  .run()
  .await;
}
