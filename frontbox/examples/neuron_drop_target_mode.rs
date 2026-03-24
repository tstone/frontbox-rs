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

  let mut io_network = IoNetworkBuilder::new();

  io_network.add_board(FastIoBoards::io_3208());

  io_network.add_board(
    FastIoBoards::io_1616()
      .with_switch_cfg(
        switches::LOWER_DROP_TARGET1,
        5,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_switch_cfg(
        switches::LOWER_DROP_TARGET2,
        6,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_switch_cfg(
        switches::LOWER_DROP_TARGET3,
        7,
        SwitchConfig {
          inverted: true,
          debounce_open: Some(Duration::from_millis(10)),
          ..Default::default()
        },
      )
      .with_driver_cfg(
        drivers::LOWER_DROP_TARGET_COIL,
        3,
        PulseMode {
          trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
          initial_pwm_length: Duration::from_millis(250),
          initial_pwm_power: Power::FULL,
          ..Default::default()
        },
      ),
  );

  App::boot(BootConfig::default(), io_network.build(), vec![])
    .await
    .plugin(OperationalPlugin)
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

  fn on_switch_closed(&mut self, switch: &Switch, ctx: &mut Context) {
    if self.target_switches.contains(&switch.name) {
      let switch_lookup = ctx.expect::<SwitchLookup>();

      let all_down = self
        .target_switches
        .iter()
        .all(|&target| switch_lookup.is_closed(target).unwrap_or(false));

      if all_down {
        ctx.cue(Action, Cue::Once(Duration::from_millis(250)));
      }
    }
  }
}

impl System for DropTargetDownUp {
  fn on_startup(&mut self, ctx: &mut Context) {
    // bring up all targets on startup
    ctx.command(ActivateDriver::new(
      drivers::LOWER_DROP_TARGET_COIL,
      ActivationMode::Tap,
    ));
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      self.on_switch_closed(&event.switch, ctx);
    }
  }

  fn on_cue(&mut self, _cue: &dyn Signal, ctx: &mut Context) {
    ctx.command(ActivateDriver::new(
      drivers::START_BUTTON_LAMP,
      ActivationMode::Tap,
    ));
  }
}
