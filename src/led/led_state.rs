use std::time::Duration;

use palette::Srgb;

use crate::led::animation::Animation;

pub enum LedState {
  On(Srgb),
  Off,
}

impl LedState {
  pub fn off() -> Self {
    LedState::Off
  }

  pub fn on(color: Srgb) -> Self {
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

  pub fn on(mut self, name: &'static str, color: Srgb) -> Self {
    self
      .declarations
      .push(LedDeclaration::new(name, LedState::On(color)));
    self
  }

  pub fn next_frame(
    self,
    name: &'static str,
    animation: &mut (impl Animation<Srgb> + 'static),
  ) -> Self {
    animation.tick(*self.delta_time);
    self.on(name, animation.sample())
  }

  pub fn collect(self) -> Vec<LedDeclaration> {
    self.declarations
  }
}
