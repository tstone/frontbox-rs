use std::time::Duration;

use crate::led::animation::Animation;
use crate::prelude::Color;

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

pub struct LedDeclaration {
  pub name: &'static str,
  pub state: LedState,
}

impl LedDeclaration {
  pub fn new(name: &'static str, state: LedState) -> Self {
    Self { name, state }
  }
}

pub struct LedDeclarationBuilder<'a> {
  delta_time: &'a Duration,
  declarations: Vec<LedDeclaration>,
}

impl<'a> LedDeclarationBuilder<'a> {
  pub fn new(delta_time: &'a Duration) -> Self {
    Self {
      delta_time,
      declarations: Vec::new(),
    }
  }

  pub fn off(mut self, name: &'static str) -> Self {
    self
      .declarations
      .push(LedDeclaration::new(name, LedState::Off));
    self
  }

  pub fn on(mut self, name: &'static str, color: Color) -> Self {
    self
      .declarations
      .push(LedDeclaration::new(name, LedState::On(color)));
    self
  }

  pub fn next_frame(
    self,
    name: &'static str,
    animation: &mut Box<dyn Animation<Color> + 'static>,
  ) -> Self {
    animation.tick(*self.delta_time);
    self.on(name, animation.sample())
  }

  pub fn next_frames(
    mut self,
    animation: &mut Box<dyn Animation<Vec<(&'static str, Color)>> + 'static>,
  ) -> Self {
    animation.tick(*self.delta_time);
    for (name, color) in animation.sample() {
      self = self.on(name, color);
    }
    self
  }

  pub fn collect(self) -> Vec<LedDeclaration> {
    self.declarations
  }
}
