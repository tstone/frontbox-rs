use frontbox::plugins::*;
use frontbox::prelude::*;
use frontbox::protocol::prelude::DriverConfig;
use frontbox::protocol::prelude::PulseBuilder;
use frontbox::runtimes::PlayerRuntime;
use std::io::Write;

pub mod switches {
  pub const START_BUTTON: &str = "start_button";
  pub const LEFT_FLIPPER_BUTTON: &str = "left_flipper_button";
  pub const RIGHT_FLIPPER_BUTTON: &str = "right_flipper_button";
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

  io_network.add_board(FastIoBoards::io_3208());

  io_network.add_board(
    FastIoBoards::io_1616()
      .with_switch(5, switches::LOWER_DROP_TARGET1)
      .with_switch_config(
        switches::LOWER_DROP_TARGET1,
        SwitchConfig {
          inverted: true,
          ..Default::default()
        },
      )
      .with_switch(6, switches::LOWER_DROP_TARGET2)
      .with_switch_config(
        switches::LOWER_DROP_TARGET2,
        SwitchConfig {
          inverted: true,
          ..Default::default()
        },
      )
      .with_switch(7, switches::LOWER_DROP_TARGET3)
      .with_switch_config(
        switches::LOWER_DROP_TARGET3,
        SwitchConfig {
          inverted: true,
          ..Default::default()
        },
      )
      .with_driver(0, drivers::LOWER_DROP_TARGET_COIL)
      .with_driver_config(
        drivers::LOWER_DROP_TARGET_COIL,
        DriverConfig::pulse().build(),
      ),
  );

  MachineBuilder::boot(
    BootConfig {
      platform: FastPlatform::Neuron,
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      ..Default::default()
    },
    io_network.build(),
  )
  .await
  .add_keyboard_mappings(vec![
    (KeyCode::Char('1'), switches::LOWER_DROP_TARGET1),
    (KeyCode::Char('2'), switches::LOWER_DROP_TARGET2),
    (KeyCode::Char('3'), switches::LOWER_DROP_TARGET3),
  ])
  .add_virtual_switch(KeyCode::Home, switches::START_BUTTON)
  .build()
  .run(PlayerRuntime::new(vec![DropTargetDownUp::new([
    switches::LOWER_DROP_TARGET1,
    switches::LOWER_DROP_TARGET2,
    switches::LOWER_DROP_TARGET3,
  ])]))
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

impl System for DropTargetDownUp {
  fn on_game_start(&mut self, ctx: &mut Context) {
    ctx.trigger_driver(
      drivers::LOWER_DROP_TARGET_COIL,
      DriverTriggerControlMode::Manual,
    );
  }

  fn on_switch_closed(&mut self, switch: &Switch, ctx: &mut Context) {
    if self.target_switches.contains(&switch.name) {
      // each target down gets points
      // ctx.command(AddPoints(100));

      let all_down = self
        .target_switches
        .iter()
        .all(|&target| ctx.is_switch_closed(target).unwrap_or(false));

      if all_down {
        // ctx.command(AddPoints(1000));
        ctx.trigger_driver(
          drivers::LOWER_DROP_TARGET_COIL,
          DriverTriggerControlMode::Manual,
        );
      }
    }
  }
}
