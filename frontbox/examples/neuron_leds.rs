use frontbox::animation::*;
use frontbox::prelude::*;
use std::io::Write;

/**
 * This example demonstrates how to use the animation system to various LED effects
 */

pub mod leds {
  pub const DEMO1: &str = "demo1";
  pub const DEMO2: &str = "demo2";
  pub const DEMO3: &str = "demo3";
  pub const DEMO4: &str = "demo4";
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let expansion_boards = vec![
    ExpansionBoard::neutron().port(
      0,
      LedPort::ws2812()
        // like other hardware, LEDs can be arbitrary tagged for later querying
        .with(led(leds::DEMO1).tagged(tags::ActionButton))
        .with(led(leds::DEMO2))
        .with(led(leds::DEMO3))
        .with(led(leds::DEMO4)),
    ),
  ];

  App::boot(
    BootConfig::default(),
    IoNetworkBuilder::new().build(),
    expansion_boards,
  )
  .await
  .configure(|app| {
    app.system(LedExample::new());
  })
  .run()
  .await;
}

struct LedExample {
  flash: Box<dyn Animation<Duration, Color>>,
  seq: Box<dyn Animation<Duration, Color>>,
}

impl LedExample {
  fn new() -> Self {
    Self {
      flash: Tween::boxed(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Color::black(), Color::purple()],
        AnimationCycle::Forever,
      ),
      seq: Sequence::boxed(
        vec![
          Tween::boxed(
            Duration::from_millis(1500),
            Curve::QuadraticInOut,
            vec![Color::black(), Color::red()],
            AnimationCycle::Once,
          ),
          Tween::boxed(
            Duration::from_millis(200),
            Curve::Sinusoid,
            vec![Color::red(), Color::yellow()],
            AnimationCycle::Once,
          ),
          Tween::boxed(
            Duration::from_millis(400),
            Curve::Linear,
            vec![Color::yellow(), Color::black()],
            AnimationCycle::Once,
          ),
        ],
        AnimationCycle::Forever,
      ),
    }
  }
}

impl System for LedExample {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    let mut leds = systems.expect_mut::<LedSystem>();

    // set static LED colors on demand
    leds.declare(
      ctx.current_system_id(),
      named_led(ctx, leds::DEMO1).color(Color::blue()),
    );
    leds.declare(
      ctx.current_system_id(),
      named_led(ctx, leds::DEMO2).color(Color::dark_blue()),
    );
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
    // tick animations to update their internal state
    self.flash.accumulate(delta);
    self.seq.accumulate(delta);

    let mut leds = systems.expect_mut::<LedSystem>();

    // re-declare LEDs with updated animated colors
    leds.declare(
      ctx.current_system_id(),
      named_led(ctx, leds::DEMO3).color(self.flash.sample()),
    );
    leds.declare(
      ctx.current_system_id(),
      named_led(ctx, leds::DEMO4).color(self.seq.sample()),
    );
  }
}
