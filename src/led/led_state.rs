use std::collections::HashMap;
use std::time::Duration;

use crate::led::animation::Animation;
use crate::prelude::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum LedState {
  On(Color),
  Off,
}

impl LedState {
  pub fn off() -> Self {
    LedState::Off
  }

  pub fn on(color: Color) -> Self {
    LedState::On(color)
  }
}

pub struct LedDeclarationBuilder {
  delta_time: Duration,
  declarations: HashMap<&'static str, LedState>,
}

impl LedDeclarationBuilder {
  pub fn new(delta_time: Duration) -> Self {
    Self {
      delta_time,
      declarations: HashMap::new(),
    }
  }

  pub fn empty() -> HashMap<&'static str, LedState> {
    HashMap::new()
  }

  pub fn off(mut self, name: &'static str) -> Self {
    self.declarations.insert(name, LedState::Off);
    self
  }

  pub fn on(mut self, name: &'static str, color: Color) -> Self {
    self.declarations.insert(name, LedState::On(color));
    self
  }

  pub fn next_frame(
    self,
    name: &'static str,
    animation: &mut Box<dyn Animation<Color> + 'static>,
  ) -> Self {
    animation.tick(self.delta_time);
    self.on(name, animation.sample())
  }

  pub fn next_frames(
    mut self,
    animation: &mut Box<dyn Animation<Vec<(&'static str, Color)>> + 'static>,
  ) -> Self {
    animation.tick(self.delta_time);
    for (name, color) in animation.sample() {
      self = self.on(name, color);
    }
    self
  }

  pub fn collect(self) -> HashMap<&'static str, LedState> {
    self.declarations
  }
}
