use fast_protocol::Color;

use crate::prelude::LedDeclarationBuilder;

#[derive(Clone)]
pub enum LedSetting {
  On(Color),
  Off,
}

impl LedSetting {
  pub fn off() -> Self {
    LedSetting::Off
  }

  pub fn on(color: Color) -> Self {
    LedSetting::On(color)
  }

  pub fn is_on(&self) -> bool {
    matches!(self, LedSetting::On(_))
  }

  pub fn is_off(&self) -> bool {
    matches!(self, LedSetting::Off)
  }

  pub fn add_declaration(
    &mut self,
    builder: LedDeclarationBuilder,
    led_name: &'static str,
  ) -> LedDeclarationBuilder {
    if let LedSetting::On(color) = self {
      return builder.on(led_name, *color);
    } else {
      return builder.off(led_name);
    }
  }
}
