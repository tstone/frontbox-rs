use crate::prelude::*;

/// Given a set of multiple states for the same LED, resolve the current state
pub trait LedResolver {
  fn resolve(&mut self, name: &'static str, states: Vec<(u64, LedState)>) -> LedState;
  fn tick(&mut self, _delta: Duration) {}
  fn reset(&mut self) {}
}
