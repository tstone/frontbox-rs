use frontbox::prelude::*;
use frontbox::runtimes::AttractMode;
use std::io::Write;

pub mod leds {
  pub const LEFT_LANE: &str = "left_lane";
  pub const CENTER_LANE: &str = "center_lane";
  pub const RIGHT_LANE: &str = "right_lane";
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
    leds: vec![leds::LEFT_LANE, leds::CENTER_LANE, leds::RIGHT_LANE],
  })];

  MachineBuilder::boot(
    BootConfig::default(),
    IoNetworkSpec::new().build(),
    expansion_boards,
  )
  .await
  .build()
  .run(AttractMode::new(vec![]))
  .await;
}
