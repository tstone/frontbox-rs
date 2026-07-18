use frontbox::prelude::*;
use std::io::Write;

/**
 * This example demonstrates how blending modes interact with two competing systems trying to control the same LED.
 * System 1 and System 2 both flash the same LED on and off, but at different rates. When they are both on, the
 * resolver mode kicks in.
 */

pub mod leds {
  use super::*;

  hardware_defs! {
    pub DEMO1: LedDefinition = LedDefinition::single("demo1");
  }
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let expansion_boards =
    vec![ExpansionBoard::neuron().wire_led_port(0, LedPort::ws2812().leds(vec![&leds::DEMO1]))];

  App::boot(BootConfig {
    exp_network: expansion_boards,
    ..Default::default()
  })
  .await
  .configure(|app| {
    app.system(LedSystem::new());
    app.system(System1);
    app.system(System2);
  })
  .run()
  .await;
}

/// This implementation flashes an LED by pre-declaring an LED state then using activate/deactivate for flashing. This is efficient approach since it re-uses the same declaration.
struct System1;

impl System for System1 {
  fn on_spawn(&mut self, ctx: &Context) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_secs(4)));
    ctx.declare_leds(&leds::DEMO1.q(), Rgba::red());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if event.is::<On>() {
      ctx.set_led_conflict_resolution(&leds::DEMO1.q(), LedConflictResolution::Alternate);
    } else if event.is::<Off>() {
      ctx.set_led_conflict_resolution(&leds::DEMO1.q(), LedConflictResolution::Mix);
    }
  }
}

struct System2;

impl System for System2 {
  fn on_spawn(&mut self, ctx: &Context) {
    ctx.declare_leds(&leds::DEMO1.q(), Rgba::blue());
  }
}
