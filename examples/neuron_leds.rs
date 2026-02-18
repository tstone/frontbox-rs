use frontbox::prelude::*;
use frontbox::runtimes::AttractMode;
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

  let expansion_boards = vec![ExpansionBoardSpec::neutron().with_led_port(LedPortSpec {
    port: 0,
    start: 0,
    led_type: LedType::WS2812,
    leds: vec![leds::DEMO1, leds::DEMO2, leds::DEMO3, leds::DEMO4],
  })];

  MachineBuilder::boot(
    BootConfig::default(),
    IoNetworkSpec::new().build(),
    expansion_boards,
  )
  .await
  .build()
  .run(AttractMode::new(vec![LedExample::new()]))
  .await;
}

#[derive(Clone)]
struct LedExample {
  flash: Box<dyn Animation<Color>>,
  seq: Box<dyn Animation<Color>>,
}

impl LedExample {
  fn new() -> Box<Self> {
    Box::new(Self {
      flash: InterpolationAnimation::new(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Color::black(), Color::purple()],
        AnimationCycle::Forever,
      ),
      seq: SequenceAnimation::new(
        vec![
          InterpolationAnimation::new(
            Duration::from_millis(1500),
            Curve::QuadraticInOut,
            vec![Color::black(), Color::red()],
            AnimationCycle::Once,
          ),
          InterpolationAnimation::new(
            Duration::from_millis(200),
            Curve::Sinusoid,
            vec![Color::red(), Color::yellow()],
            AnimationCycle::Once,
          ),
          InterpolationAnimation::new(
            Duration::from_millis(400),
            Curve::Linear,
            vec![Color::yellow(), Color::black()],
            AnimationCycle::Once,
          ),
        ],
        AnimationCycle::Forever,
      ),
    })
  }
}

impl System for LedExample {
  fn leds(&mut self, delta_time: Duration) -> HashMap<&'static str, LedState> {
    LedDeclarationBuilder::new(delta_time)
      .on(leds::DEMO1, Color::deep_sky_blue())
      .on(leds::DEMO2, Color::dark_blue())
      .next_frame(leds::DEMO3, &mut self.flash)
      .next_frame(leds::DEMO4, &mut self.seq)
      .collect()
  }
}
