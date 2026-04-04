use frontbox::animation::*;
use frontbox::prelude::*;
use std::collections::HashMap;
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
    ExpansionBoard::neutron()
      .port(
        0,
        LedPort::ws2812()
          .with(led(leds::DEMO1).tagged(tags::ActionButton))
          .with(led(leds::DEMO2).coords(15.5, 10.3))
          .with(led(leds::DEMO3).coords(1.0, 1.0))
          .with(led(leds::DEMO4).tagged(tags::StartButton).coords(5.0, 5.0)),
      )
      .port(
        1,
        LedPort::ws2812().with(led_strip("example", 8).coords(7, 1.45, 3.8)),
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

#[derive(Clone)]
struct LedExample {
  flash: Box<dyn Animation<Duration, Color>>,
  seq: Box<dyn Animation<Duration, Color>>,
}

impl LedExample {
  fn new() -> Self {
    Self {
      flash: Tween::new(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Color::black(), Color::purple()],
        AnimationCycle::Forever,
      ),
      seq: Sequence::new(
        vec![
          Tween::new(
            Duration::from_millis(1500),
            Curve::QuadraticInOut,
            vec![Color::black(), Color::red()],
            AnimationCycle::Once,
          ),
          Tween::new(
            Duration::from_millis(200),
            Curve::Sinusoid,
            vec![Color::red(), Color::yellow()],
            AnimationCycle::Once,
          ),
          Tween::new(
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
  fn leds(
    &mut self,
    delta_time: Duration,
    _ctx: &Context,
    _systems: &Systems,
  ) -> HashMap<&'static str, LedState> {
    LedDeclarationBuilder::new(delta_time)
      .on(leds::DEMO1, Color::deep_sky_blue())
      .on(leds::DEMO2, Color::dark_blue())
      .next_frame(leds::DEMO3, &mut self.flash)
      .next_frame(leds::DEMO4, &mut self.seq)
      .collect()
  }
}
