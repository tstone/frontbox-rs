use fast_protocol::LedType;

use crate::prelude::*;

#[derive(Debug)]
pub struct LedPort {
  pub led_type: LedType,
  pub leds: Vec<&'static LedDefinition>,
}

impl LedPort {
  pub fn new(led_type: LedType) -> Self {
    Self {
      led_type,

      leds: Vec::new(),
    }
  }

  pub fn ws2812() -> Self {
    Self::new(LedType::WS2812)
  }

  pub fn sk6812() -> Self {
    Self::new(LedType::SK6812)
  }

  pub fn apa102() -> Self {
    Self::new(LedType::APA102)
  }

  pub fn leds(mut self, leds: Vec<&'static LedDefinition>) -> Self {
    self.leds = leds;
    self
  }
}

#[derive(Debug, Clone)]
pub struct ResolvedLedPort {
  pub led_type: LedType,
  pub leds: Vec<ExpAddressed<SingleLedDefinition>>,
  pub offset: u16,
  pub length: u8,
}

impl ResolvedLedPort {
  pub fn default(offset: u16) -> Self {
    Self {
      led_type: LedType::WS2812,
      leds: Vec::new(),
      offset,
      length: 32,
    }
  }
}
