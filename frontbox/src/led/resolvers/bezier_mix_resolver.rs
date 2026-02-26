use crate::prelude::*;
use fast_protocol::Color;

/// Resolves LED conflicts by mixing all states in the list
pub struct BezierMixResolver;

impl BezierMixResolver {
  pub fn new() -> Self {
    Self
  }

  fn normalize_color(a: (u64, LedState)) -> Color {
    match a.1 {
      LedState::On(c) => c,
      LedState::Off => Color::default(),
    }
  }

  fn mix_pair(a: (u64, LedState), b: (u64, LedState)) -> LedState {
    let c1 = Self::normalize_color(a);
    let c2 = Self::normalize_color(b);
    let cr = c1.mix(&c2, 0.5);

    if cr != Color::default() {
      return LedState::On(cr);
    } else {
      return LedState::Off;
    }
  }
}

impl LedResolver for BezierMixResolver {
  fn resolve(&mut self, _: &'static str, states: Vec<(u64, LedState)>) -> LedState {
    if states.len() == 0 {
      return LedState::Off;
    } else if states.len() == 1 {
      return states[0].1.clone();
    } else if states.len() == 2 {
      Self::mix_pair(states[0].clone(), states[1].clone())
    } else {
      // if more than 2 colors, mix them in pairs recursively until we have one final color
      self.resolve(
        "",
        states
          .windows(2)
          .map(|chunk| (0, Self::mix_pair(chunk[0].clone(), chunk[1].clone())))
          .collect(),
      )
    }
  }
}
