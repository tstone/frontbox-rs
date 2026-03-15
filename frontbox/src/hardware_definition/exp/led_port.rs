use fast_protocol::LedType;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub struct LedPort {
  pub port: u8,
  pub start: u8,
  pub leds: Vec<&'static str>,
  pub led_type: LedType,
}

impl Default for LedPort {
  fn default() -> Self {
    Self {
      port: 0,
      start: 0,
      leds: Vec::new(),
      led_type: LedType::WS2812,
    }
  }
}
