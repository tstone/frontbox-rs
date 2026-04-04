use fast_protocol::LedType;

use crate::{Illumination, ResolvedIllumination};

#[derive(Debug)]
pub struct LedPort {
  pub led_type: LedType,
  pub illuminations: Vec<Box<dyn Illumination>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedLedPort {
  pub led_type: LedType,
  pub illuminations: Vec<ResolvedIllumination>,
  pub start: u16,
  pub length: u8,
}

impl LedPort {
  pub fn new(led_type: LedType) -> Self {
    Self {
      led_type,

      illuminations: Vec::new(),
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

  pub fn with(mut self, illumination: impl Illumination + 'static) -> Self {
    self.illuminations.push(Box::new(illumination));
    self
  }
}
