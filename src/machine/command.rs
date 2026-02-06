use std::any::Any;

use crate::prelude::*;

pub trait Command: Any + Send + Sync {
  fn uniquness(&self) -> CommandUniqueness {
    CommandUniqueness::NonExclusive
  }

  fn execute(&self, machine: &mut Machine);
}

pub enum CommandUniqueness {
  /// Command will only be executed once, first one wins
  UniqueFirst,
  /// Command will only be executed once, last one wins
  UniqueLast,
  /// Command can be executed multiple times during the same event
  NonExclusive,
}
