use frontbox::prelude::*;
use frontbox::tags::Playfield;

use std::io::Write;
use std::time::Duration;

pub mod drivers {
  pub const LOWER_DROP_TARGET_COIL: &str = "lower_drop_target_coil";
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let target1 = SwitchDefinition::new("drop_target1")
    .tag(Playfield)
    .inverted()
    .debounce_open(Duration::from_millis(10))
    .build();

  let target2 = SwitchDefinition::new("drop_target2")
    .tag(Playfield)
    .inverted()
    .debounce_open(Duration::from_millis(10))
    .build();

  let target3 = SwitchDefinition::new("drop_target3")
    .tag(Playfield)
    .inverted()
    .debounce_open(Duration::from_millis(10))
    .build();

  let mut io_network = IoNetworkBuilder::new();
  io_network.add_board(IoBoards::io_3208());
  io_network.add_board(
    IoBoards::io_1616()
      .wire_switch(5, &target1)
      .wire_switch(6, &target2)
      .wire_switch(7, &target3)
      .with(
        driver(3)
          .named(drivers::LOWER_DROP_TARGET_COIL)
          .mode(PulseMode {
            trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
            initial_pwm_length: Duration::from_millis(250),
            initial_pwm_power: Power::FULL,
            ..Default::default()
          }),
      ),
  );

  App::boot("/dev/ttyACM0", "/dev/ttyACM1", io_network.build(), vec![])
    .await
    .configure(|app| {
      app.system(DropTargetDownUp::new([
        target1.name,
        target2.name,
        target3.name,
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

impl DropTargetDownUp {
  pub fn new(target_switches: [&'static str; 3]) -> Self {
    Self { target_switches }
  }

  fn on_switch_closed(&mut self, switch: &Switch, ctx: &Context) {
    if self.target_switches.contains(&switch.name) {
      let all_down = self
        .target_switches
        .iter()
        .all(|&target| ctx.switches.is_closed(target).unwrap_or(false));

      if all_down {
        ctx.cue(Action, Cue::Once(Duration::from_millis(250)));
      }
    }
  }
}

impl System for DropTargetDownUp {
  fn on_spawn(&mut self, ctx: &Context) {
    // bring up all targets on startup
    ctx.activate_driver(drivers::LOWER_DROP_TARGET_COIL, ActivationMode::Tap);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      self.on_switch_closed(&event.switch, ctx);
    }
  }
}
