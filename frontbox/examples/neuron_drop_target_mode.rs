use frontbox::prelude::*;
use frontbox::tags::Playfield;

use std::io::Write;
use std::time::Duration;

pub mod switches {
  pub const LOWER_DROP_TARGET1: &str = "lower_drop_target1";
  pub const LOWER_DROP_TARGET2: &str = "lower_drop_target2";
  pub const LOWER_DROP_TARGET3: &str = "lower_drop_target3";
}

pub mod drivers {
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
      .with(
        switch(5)
          .named(switches::LOWER_DROP_TARGET1)
          .tagged(Playfield)
          .config(SwitchConfig {
            inverted: true,
            debounce_open: Some(Duration::from_millis(10)),
            ..Default::default()
          }),
      )
      .with(
        switch(6)
          .named(switches::LOWER_DROP_TARGET2)
          .tagged(Playfield)
          .config(SwitchConfig {
            inverted: true,
            debounce_open: Some(Duration::from_millis(10)),
            ..Default::default()
          }),
      )
      .with(
        switch(7)
          .named(switches::LOWER_DROP_TARGET3)
          .tagged(Playfield)
          .config(SwitchConfig {
            inverted: true,
            debounce_open: Some(Duration::from_millis(10)),
            ..Default::default()
          }),
      )
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
        switches::LOWER_DROP_TARGET1,
        switches::LOWER_DROP_TARGET2,
        switches::LOWER_DROP_TARGET3,
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
    ctx.systems.expect::<Machine>().activate_driver(
      drivers::LOWER_DROP_TARGET_COIL,
      ActivationMode::Tap,
      ctx,
    );
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      self.on_switch_closed(&event.switch, ctx);
    }
  }
}
