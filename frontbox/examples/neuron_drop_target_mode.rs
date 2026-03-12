use frontbox::plugins::game_points::*;
use frontbox::prelude::*;

use std::io::Write;
use std::time::Duration;

pub mod switches {
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
      .with_switch_cfg(
        5,
        switches::LOWER_DROP_TARGET1,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_switch_cfg(
        6,
        switches::LOWER_DROP_TARGET2,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_switch_cfg(
        7,
        switches::LOWER_DROP_TARGET3,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_driver_cfg(
        3,
        drivers::LOWER_DROP_TARGET_COIL,
        PulseMode {
          initial_pwm_length: Duration::from_millis(250),
          initial_pwm_power: Power::FULL,
          ..Default::default()
        },
      ),
  );

  MachineBuilder::boot(BootConfig::default(), io_network.build(), vec![])
    .await
    .add_keyboard_mappings(vec![
      (KeyCode::Char('1'), switches::LOWER_DROP_TARGET1),
      (KeyCode::Char('2'), switches::LOWER_DROP_TARGET2),
      (KeyCode::Char('3'), switches::LOWER_DROP_TARGET3),
    ])
    .build()
    .run(vec![DropTargetDownUp::new([
      switches::LOWER_DROP_TARGET1,
      switches::LOWER_DROP_TARGET2,
      switches::LOWER_DROP_TARGET3,
    ])])
    .await;
}

/// Example game mode where all three drop targets must be down then the targets are reset
#[derive(Debug, Clone)]
struct DropTargetDownUp {
  target_switches: [&'static str; 3],
}

impl DropTargetDownUp {
  pub fn new(target_switches: [&'static str; 3]) -> Box<Self> {
    Box::new(Self { target_switches })
  }

  fn on_switch_closed(&mut self, switch: &Switch, ctx: &Context, cmds: &mut Commands) {
    if self.target_switches.contains(&switch.name) {
      // each target down gets points
      cmds.add_points(100);

      let all_down = self
        .target_switches
        .iter()
        .all(|&target| ctx.is_switch_closed(target).unwrap_or(false));

      if all_down {
        // ctx.command(AddPoints(1000));
        cmds.add_points(1000);
        cmds.add_bonus(1000);

        cmds.trigger_delayed_driver(
          drivers::LOWER_DROP_TARGET_COIL,
          DriverTriggerControlMode::Manual,
          Duration::from_millis(250),
        );
        cmds.replace_system(*DropTargetDownUp::new(self.target_switches));
      }
    }
  }
}

impl CloneableSystem for DropTargetDownUp {
  fn on_startup(&mut self, _ctx: &Context, cmds: &mut Commands) {
    cmds.trigger_driver(
      drivers::LOWER_DROP_TARGET_COIL,
      DriverTriggerControlMode::Manual,
    );
  }

  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &Context, cmds: &mut Commands) {
    handle_event!(event, {
      SwitchClosed => |e| { self.on_switch_closed(&e.switch, ctx, cmds); }
    });
  }
}
