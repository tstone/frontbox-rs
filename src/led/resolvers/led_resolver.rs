use crate::prelude::LedState;

/// Given a set of multiple states for the same LED, resolve the current state
pub trait LedResolver {
  fn resolve(&mut self, states: Vec<(u64, LedState)>) -> LedState;
}
