use frontbox::animation::*;
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
    app.system(System2::new());
  })
  .run()
  .await;
}

/// This implementation flashes an LED by pre-declaring an LED state then using activate/deactivate for flashing. This is efficient approach since it re-uses the same declaration.
struct System1;

impl System for System1 {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    systems.expect_mut::<LedSystem>().declare_inactive(
      ctx.current_system_id(),
      named_led(ctx, leds::DEMO1).color(Color::red()),
    );

    ctx.cue_cycling(events![On, Off], Cue::Loop(Duration::from_millis(200)));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    let mut led_system = systems.expect_mut::<LedSystem>();

    if event.is::<On>() {
      led_system.activate_by_system(ctx.current_system_id());
    } else if event.is::<Off>() {
      led_system.deactivate_by_system(ctx.current_system_id());
    }
  }
}

/// Here's another way to flash an LED, via animations. This also has the benefit of controlling the transition to look more organic.
struct System2 {
  anim: Tween<Duration, Color>,
}

impl System2 {
  fn new() -> Self {
    Self {
      anim: Tween::new(
        Duration::from_millis(1000),
        Curve::ExponentialInOut,
        vec![Color::default(), Color::blue()],
        AnimationCycle::Forever,
      ),
    }
  }
}

impl System for System2 {
  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
    self.anim.accumulate(delta);

    let decl = named_led(ctx, leds::DEMO1).color(self.anim.sample());
    systems
      .expect_mut::<LedSystem>()
      .declare(ctx.current_system_id(), decl);
  }
}
