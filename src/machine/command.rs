use std::any::Any;

use crate::prelude::*;

pub trait Command: Any + Send + Sync {
  fn uniquness(&self) -> CommandUniqueness {
    CommandUniqueness::NonExclusive
  }

  /// Execute the command, potentially modifying the machine state. The system_id is provided to allow commands to modify the system that created them, if desired.
  fn execute(&self, system_id: usize, machine: &mut Machine);
}

pub enum CommandUniqueness {
  /// Command will only be executed once, first one wins
  UniqueFirst,
  /// Command will only be executed once, last one wins
  UniqueLast,
  /// Command can be executed multiple times during the same event
  NonExclusive,
}
