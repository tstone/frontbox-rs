use fast_protocol::LedType;

#[derive(Debug, Clone)]
pub struct LedPortDefinition {
  pub port: u8,
  pub start: u8,
  pub leds: Vec<&'static str>,
  pub led_type: LedType,
}

impl Default for LedPortDefinition {
  fn default() -> Self {
    Self {
      port: 0,
      start: 0,
      leds: Vec::new(),
      led_type: LedType::WS2812,
    }
  }
}
