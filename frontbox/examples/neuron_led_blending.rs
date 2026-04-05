use frontbox::prelude::*;
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

  let expansion_boards =
    vec![ExpansionBoard::neutron().port(0, LedPort::ws2812().with(led(leds::DEMO1)))];

  App::boot(
    BootConfig::default(),
    IoNetworkBuilder::new().build(),
    expansion_boards,
  )
  .await
  .configure(|app| {
    app.system(System1);
    app.system(System2);
  })
  .run()
  .await;
}

struct System1;

impl System for System1 {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_secs(1)));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    let mut led_system = systems.expect_mut::<LedSystem>();

    if event.is::<On>() {
      led_system.declare(
        ctx.current_system_id(),
        named_led(ctx, leds::DEMO1).color(Color::red()),
      );
    } else if event.is::<Off>() {
      led_system.undeclare(ctx.current_system_id(), named_led(ctx, leds::DEMO1));
    }
  }
}

struct System2;

impl System for System2 {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_secs(1)));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    let mut led_system = systems.expect_mut::<LedSystem>();

    if event.is::<On>() {
      led_system.declare(
        ctx.current_system_id(),
        named_led(ctx, leds::DEMO1).color(Color::blue()),
      );
    } else if event.is::<Off>() {
      led_system.undeclare(ctx.current_system_id(), named_led(ctx, leds::DEMO1));
    }
  }
}
