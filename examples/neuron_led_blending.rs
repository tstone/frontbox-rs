use frontbox::prelude::*;
use frontbox::runtimes::AttractMode;
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

  let expansion_boards = vec![ExpansionBoardSpec::neutron().with_led_port(LedPortSpec {
    port: 0,
    start: 0,
    led_type: LedType::WS2812,
    leds: vec![leds::DEMO1],
  })];

  MachineBuilder::boot(
    BootConfig::default(),
    IoNetworkSpec::new().build(),
    expansion_boards,
  )
  .await
  .build()
  .run(AttractMode::new(vec![System1::new(), System2::new()]))
  .await;
}

#[derive(Clone)]
struct System1 {
  on: bool,
}

impl System1 {
  fn new() -> Box<Self> {
    Box::new(Self { on: false })
  }
}

impl System for System1 {
  fn on_system_enter(&mut self, ctx: &mut Context) {
    ctx.set_timer(
      "example_timer",
      std::time::Duration::from_secs(1),
      TimerMode::Repeating,
    );
  }

  fn on_timer(&mut self, timer_name: &'static str, _ctx: &mut Context) {
    if timer_name == "example_timer" {
      self.on = !self.on;
    }
  }

  fn leds(&mut self, delta_time: Duration) -> HashMap<&'static str, LedState> {
    if self.on {
      LedDeclarationBuilder::new(delta_time)
        .on(leds::DEMO1, Color::blue())
        .collect()
    } else {
      LedDeclarationBuilder::empty()
    }
  }
}

#[derive(Clone)]
struct System2 {
  on: bool,
}

impl System2 {
  fn new() -> Box<Self> {
    Box::new(Self { on: false })
  }
}

impl System for System2 {
  fn on_system_enter(&mut self, ctx: &mut Context) {
    ctx.set_timer(
      "example_timer",
      std::time::Duration::from_millis(1500),
      TimerMode::Repeating,
    );
  }

  fn on_timer(&mut self, timer_name: &'static str, _ctx: &mut Context) {
    if timer_name == "example_timer" {
      self.on = !self.on;
    }
  }

  fn leds(&mut self, delta_time: Duration) -> HashMap<&'static str, LedState> {
    if self.on {
      LedDeclarationBuilder::new(delta_time)
        .on(leds::DEMO1, Color::red())
        .collect()
    } else {
      LedDeclarationBuilder::empty()
    }
  }
}
