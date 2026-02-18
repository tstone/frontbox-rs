use frontbox::prelude::*;
use frontbox::runtimes::AttractMode;
use palette::Srgb;
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
struct LedExample;

impl LedExample {
  fn new() -> Box<Self> {
    Box::new(Self {})
  }
}

impl System for LedExample {
  fn leds(&self, delta_time: &Duration) -> Vec<LedDeclaration> {
    LedDeclarationBuilder::new(delta_time)
      .on(leds::DEMO1, Srgb::new(1.0, 0.0, 0.0))
      .on(leds::DEMO2, Srgb::new(0.0, 1.0, 0.0))
      .on(leds::DEMO3, Srgb::new(0.0, 0.0, 1.0))
      .collect()
  }
}
