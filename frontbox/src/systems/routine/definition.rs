use std::any::{TypeId, type_name};

use crate::prelude::*;

/// The participants of a routine, in order. Usually built with [`routine!`](crate::routine).
#[derive(Debug, Clone, Default)]
pub struct RoutineDefinition {
  participants: Vec<Participant>,
}

impl RoutineDefinition {
  pub fn new() -> Self {
    Self::default()
  }

  /// Add `T` as the next participant
  pub fn then<T: System + 'static>(mut self) -> Self {
    self.participants.push(Participant {
      type_id: TypeId::of::<T>(),
      name: type_name::<T>(),
    });
    self
  }

  pub(super) fn participant(&self, index: usize) -> Option<Participant> {
    self.participants.get(index).copied()
  }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Participant {
  pub type_id: TypeId,
  pub name: &'static str,
}
