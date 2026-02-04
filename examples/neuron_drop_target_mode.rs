use frontbox::prelude::*;
use std::io::Write;
use std::time::Duration;

pub mod switches {
  pub const START_BUTTON: &str = "start_button";
  pub const LOWER_DROP_TARGET1: &str = "lower_drop_target1";
  pub const LOWER_DROP_TARGET2: &str = "lower_drop_target2";
  pub const LOWER_DROP_TARGET3: &str = "lower_drop_target3";
}

pub mod drivers {
  pub const START_BUTTON_LAMP: &str = "start_button_lamp";
  pub const LOWER_DROP_TARGET_COIL: &str = "lower_drop_target_coil";
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
      .with_switch_config(
        switches::START_BUTTON,
        SwitchConfig {
          inverted: false,
          debounce_close: Some(Duration::from_millis(20)),
          ..Default::default()
        },
      )
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
  .add_machine_scene(vec![Freeplay::new(switches::START_BUTTON, 4)])
  .add_keyboard_mappings(vec![
    (KeyCode::Char('1'), switches::LOWER_DROP_TARGET1),
    (KeyCode::Char('2'), switches::LOWER_DROP_TARGET2),
    (KeyCode::Char('3'), switches::LOWER_DROP_TARGET3),
  ])
  .add_game_scene(vec![DropTargetDownUp::new([
    switches::LOWER_DROP_TARGET1,
    switches::LOWER_DROP_TARGET2,
    switches::LOWER_DROP_TARGET3,
  ])])
  .run()
  .await;
}

/// Example game mode mode where all three drop targets must be down then the targets are reset
#[derive(Debug, Clone)]
struct DropTargetDownUp {
  target_switches: [&'static str; 3],
}

impl DropTargetDownUp {
  pub fn new(target_switches: [&'static str; 3]) -> Box<Self> {
    Box::new(Self { target_switches })
  }
}

impl GameMode for DropTargetDownUp {
  fn event_switch_closed(&mut self, switch: &Switch, ctx: &mut MachineContext) {
    if self.target_switches.contains(&switch.name) {
      // each target down gets points
      ctx.add_points(100);

      let all_down = self
        .target_switches
        .iter()
        .all(|&target| ctx.is_switch_closed(target).unwrap_or(false));

      if all_down {
        ctx.add_points(1000);
        ctx.trigger_driver(drivers::LOWER_DROP_TARGET_COIL);
      }
    }
  }
}
