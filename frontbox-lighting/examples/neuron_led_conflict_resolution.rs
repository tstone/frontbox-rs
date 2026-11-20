use frontbox_core::prelude::*;
use frontbox_lighting::prelude::*;
use std::io::Write;

/**
 * This example demonstrates how blending modes interact with two competing systems trying to control the same light.
 * System 1 and System 2 both flash the same light on and off, but at different rates. When they are both on, the
 * resolver mode kicks in.
 */

pub mod lights {
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

  let exp_network = ExpNetwork::new(vec![
    ExpBoard::neuron().wire_led_port(1, LedPort::ws2812().leds(vec![&lights::DEMO1])),
  ]);

  App::new(BootConfig {
    exp_network,
    ..Default::default()
  })
  .configure(|app| {
    app.system(LayeredLightManager::new());
    app.system(System1);
    app.system(System2);
  })
  .run()
  .await;
}

/// This implementation flashes a light by pre-declaring a light state then using activate/deactivate for flashing. This is efficient approach since it re-uses the same declaration.
struct System1;

#[derive(serde::Serialize, Event)]
pub struct On;
#[derive(serde::Serialize, Event)]
pub struct Off;

impl System for System1 {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx
      .timers
      .start_multi(events![On, Off], Timer::Forever(Duration::from_secs(4)));
    ctx.declare_lights(&lights::DEMO1.q(), 0, Rgba::red());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<On>() {
      ctx.set_light_conflict_resolution(&lights::DEMO1.q(), LightConflictResolution::Alternate);
    } else if event.is::<Off>() {
      ctx.set_light_conflict_resolution(&lights::DEMO1.q(), LightConflictResolution::Mix);
    }
  }
}

struct System2;

impl System for System2 {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.declare_lights(&lights::DEMO1.q(), 0, Rgba::blue());
  }
}
