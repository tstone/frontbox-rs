use crossterm::style::Color;

use crate::led::animation::Animation;

pub enum LedState {
  On(Color),
  Off,
  Animated(Box<dyn Animation<Color>>),
}

impl LedState {
  pub fn off() -> Self {
    LedState::Off
  }

  pub fn on(color: Color) -> Self {
    LedState::On(color)
  }

  pub fn animated(animation: Box<dyn Animation<Color>>) -> Self {
    LedState::Animated(animation)
  }
}

pub enum LedDeclaration {
  Single(LedState),
}
