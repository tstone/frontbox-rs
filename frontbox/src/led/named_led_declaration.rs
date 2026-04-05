use fast_protocol::Color;

use crate::prelude::*;

pub fn named_led(ctx: &Context, name: &str) -> NamedLed {
  let ill = ctx.illuminations.get(name).expect("LED not found");
  let led = ill
    .leds
    .first()
    .expect("LED has no addressable LEDs")
    .clone();
  NamedLed { led, z_index: None }
}

pub struct NamedLed {
  led: AddressableLed,
  z_index: Option<i8>,
}

impl NamedLed {
  pub fn color(self, color: Color) -> NamedLedDeclaration {
    NamedLedDeclaration {
      led: self.led,
      color,
      z_index: self.z_index,
    }
  }

  pub fn z_index(mut self, z: i8) -> Self {
    self.z_index = Some(z);
    self
  }
}

impl From<NamedLed> for LedIdentifications {
  fn from(named: NamedLed) -> Self {
    LedIdentifications::new(vec![named.led], named.z_index.unwrap_or(0))
  }
}

pub struct NamedLedDeclaration {
  led: AddressableLed,
  color: Color,
  z_index: Option<i8>,
}

impl From<NamedLedDeclaration> for LedDeclarations {
  fn from(decl: NamedLedDeclaration) -> Self {
    LedDeclarations::new(
      vec![(decl.led, Some(decl.color))],
      decl.z_index.unwrap_or(0),
    )
  }
}

impl From<NamedLedDeclaration> for LedIdentifications {
  fn from(decl: NamedLedDeclaration) -> Self {
    LedIdentifications::new(vec![decl.led], decl.z_index.unwrap_or(0))
  }
}
