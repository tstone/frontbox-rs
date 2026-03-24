use crate::prelude::*;

/// Given a set of multiple states for the same LED, resolve the current state
pub trait LedResolver: Send + Sync {
  fn resolve(&mut self, name: &'static str, colors: Vec<(u64, Color)>) -> Color;
  fn tick(&mut self, _delta: Duration) {}
  fn reset(&mut self) {}
}
