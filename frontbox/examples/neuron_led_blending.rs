use frontbox::prelude::*;
use std::collections::HashMap;
use std::io::Write;

/**
 * This example demonstrates how blending modes interact with two competing systems trying to control the same LED.
 * System 1 and System 2 both flash the same LED on and off, but at different rates. When they are both on, the
 * resolver mode kicks in.
 */

pub mod leds {
  pub const DEMO1: &str = "demo1";
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let expansion_boards = vec![ExpansionBoard::neutron().with_led_port(LedPort {
    port: 0,
    start: 0,
    led_type: LedType::WS2812,
    leds: vec![leds::DEMO1],
  })];

  App::boot(
    BootConfig::default(),
    IoNetworkBuilder::new().build(),
    expansion_boards,
  )
  .await
  .configure(|app| {
    app.system(System1::new());
    app.system(System2::new());
  })
  .run()
  .await;
}

struct System1 {
  on: bool,
}

impl System1 {
  fn new() -> Self {
    Self { on: false }
  }
}

impl System for System1 {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_secs(1)));
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &Context, _systems: &Systems) {
    if let Some(_) = event.downcast_ref::<On>() {
      self.on = true;
    } else if let Some(_) = event.downcast_ref::<Off>() {
      self.on = false;
    }
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    _ctx: &Context,
    _systems: &Systems,
  ) -> HashMap<&'static str, LedState> {
    if self.on {
      LedDeclarationBuilder::new(delta_time)
        .on(leds::DEMO1, Color::blue())
        .collect()
    } else {
      LedDeclarationBuilder::empty()
    }
  }
}

struct System2 {
  on: bool,
}

impl System2 {
  fn new() -> Self {
    Self { on: false }
  }
}

impl System for System2 {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_secs(2)));
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &Context, _systems: &Systems) {
    if let Some(_) = event.downcast_ref::<On>() {
      self.on = true;
    } else if let Some(_) = event.downcast_ref::<Off>() {
      self.on = false;
    }
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    _ctx: &Context,
    _systems: &Systems,
  ) -> HashMap<&'static str, LedState> {
    if self.on {
      LedDeclarationBuilder::new(delta_time)
        .on(leds::DEMO1, Color::red())
        .collect()
    } else {
      LedDeclarationBuilder::empty()
    }
  }
}
