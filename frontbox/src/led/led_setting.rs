use fast_protocol::Color;

use crate::animation::Animation;
use crate::prelude::LedDeclarationBuilder;

#[derive(Clone)]
pub enum LedSetting {
  Animation(Box<dyn Animation<Color> + 'static>),
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

  pub fn animation(animation: impl Animation<Color> + 'static) -> Self {
    LedSetting::Animation(Box::new(animation))
  }

  pub fn is_animation(&self) -> bool {
    matches!(self, LedSetting::Animation(_))
  }

  pub fn is_on(&self) -> bool {
    matches!(self, LedSetting::On(_))
  }

  pub fn is_off(&self) -> bool {
    matches!(self, LedSetting::Off)
  }

  pub fn boxed_animation(&mut self) -> Option<&mut Box<dyn Animation<Color>>> {
    if let LedSetting::Animation(anim) = self {
      Some(anim)
    } else {
      None
    }
  }

  pub fn add_declaration(
    &mut self,
    builder: LedDeclarationBuilder,
    led_name: &'static str,
  ) -> LedDeclarationBuilder {
    if let Some(animation) = self.boxed_animation() {
      return builder.next_frame(led_name, animation);
    } else if let LedSetting::On(color) = self {
      return builder.on(led_name, *color);
    } else {
      return builder.off(led_name);
    }
  }
}
