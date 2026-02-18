use frontbox::prelude::*;
use frontbox::runtimes::AttractMode;
use std::io::Write;

pub mod leds {
  pub const DEMO1: &str = "demo1";
  pub const DEMO2: &str = "demo2";
  pub const DEMO3: &str = "demo3";
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
    leds: vec![leds::DEMO1, leds::DEMO2, leds::DEMO3],
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
      // on/off flash animation that flashes magenta-ish
      flash: InterpolationAnimation::new(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        Color::black(),
        Color::purple(),
        AnimationCycle::Forever,
      ),
      seq: SequenceAnimation::new(
        vec![
          InterpolationAnimation::new(
            Duration::from_millis(1000),
            Curve::ExponentialInOut,
            Color::red(),
            Color::orange(),
            AnimationCycle::Once,
          ),
          InterpolationAnimation::new(
            Duration::from_millis(1000),
            Curve::ExponentialInOut,
            Color::orange(),
            Color::yellow(),
            AnimationCycle::Once,
          ),
          InterpolationAnimation::new(
            Duration::from_millis(1000),
            Curve::ExponentialInOut,
            Color::yellow(),
            Color::blue(),
            AnimationCycle::Once,
          ),
        ],
        AnimationCycle::Forever,
      ),
    })
  }
}

impl System for LedExample {
  fn leds(&mut self, delta_time: &Duration) -> Vec<LedDeclaration> {
    LedDeclarationBuilder::new(delta_time)
      .on(leds::DEMO1, Color::green())
      .next_frame(leds::DEMO2, &mut self.flash)
      .next_frame(leds::DEMO3, &mut self.seq)
      .collect()
  }
}
