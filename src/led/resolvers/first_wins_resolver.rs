use crate::prelude::*;

/// Resolves LED conflicts by always taking the first state in the list
pub struct FirstWinsResolver;

impl FirstWinsResolver {
  pub fn new() -> Self {
    Self
  }
}

impl LedResolver for FirstWinsResolver {
  fn resolve(&mut self, states: Vec<(u64, LedState)>) -> LedState {
    if states.len() > 0 {
      states[0].1.clone()
    } else {
      LedState::Off
    }
  }
}
