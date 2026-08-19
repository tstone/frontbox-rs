use frontbox::prelude::*;
use frontbox::tags::Playfield;

use std::io::Write;
use std::time::Duration;

use crate::drivers::*;

pub mod drivers {
  use super::*;

  hardware_defs! {
    pub BANK_COIL: DriverDefinition = DriverDefinition::new("drop_coil")
      .mode(PulseMode {
        trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
        initial_pwm_length: HardwareValue::fixed(Duration::from_millis(250)),
        initial_pwm_power: HardwareValue::fixed(Power::FULL),
        ..Default::default()
      });

    pub TARGET_1: SwitchDefinition = SwitchDefinition::new("drop_target1")
      .tag(Playfield)
      .inverted()
      .debounce_open(Duration::from_millis(10));

    pub TARGET_2: SwitchDefinition = SwitchDefinition::new("drop_target2")
      .tag(Playfield)
      .inverted()
      .debounce_open(Duration::from_millis(10));

    pub TARGET_3: SwitchDefinition = SwitchDefinition::new("drop_target3")
      .tag(Playfield)
      .inverted()
      .debounce_open(Duration::from_millis(10));

  }
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let io_network = IoNetwork::new(vec![
    IoBoards::io_3208(),
    IoBoards::io_1616()
      .wire_switch(5, &TARGET_1)
      .wire_switch(6, &TARGET_2)
      .wire_switch(7, &TARGET_3)
      .wire_driver(3, &drivers::BANK_COIL),
  ]);

  App::boot(BootConfig {
    io_network,
    ..Default::default()
  })
  .await
  .configure(|app| {
    app.system(DropTargetDownUp::new([
      TARGET_1.name,
      TARGET_2.name,
      TARGET_3.name,
    ]));
  })
  .run()
  .await;
}

/// Example game mode where all three drop targets must be down then the targets are reset
#[derive(Debug, Clone)]
struct DropTargetDownUp {
  target_switches: [&'static str; 3],
}

#[derive(serde::Serialize, Event)]
struct DropUp; // cue to send self

impl DropTargetDownUp {
  pub fn new(target_switches: [&'static str; 3]) -> Self {
    Self { target_switches }
  }

  fn on_switch_closed(&mut self, switch: &Switch, ctx: &SystemContext) {
    if self.target_switches.contains(&switch.name) {
      let all_down = self
        .target_switches
        .iter()
        .all(|&target| ctx.switches.is_closed(target).unwrap_or(false));

      if all_down {
        ctx.cue(DropUp, Cue::Once(Duration::from_millis(250)));
      }
    }
  }

  pub fn up(&self, _ctx: &SystemContext) {
    // TODO
  }
}

impl System for DropTargetDownUp {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // bring up all targets on startup
    ctx.activate_driver(drivers::BANK_COIL.name, ActivationMode::Tap);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      self.on_switch_closed(&event.switch, ctx);
    } else if event.is::<DropUp>() {
      self.up(ctx);
    }
  }
}
