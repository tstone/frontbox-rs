use std::any::TypeId;
use std::fmt::Debug;

use crate::app::event_box::EventBox;
use crate::prelude::*;

/// A request to start a routine, sent from a context to the run loop
pub struct RoutineStart {
  pub(crate) origin: SystemHandle,
  pub(crate) type_id: TypeId,
  pub(crate) name: &'static str,
  pub(crate) routine: Box<dyn Event>,
  /// Copies the routine into its `RoutineStarted`. Chosen here, while its type is still known
  pub(crate) into_started: fn(&dyn Event) -> EventBox,
  /// Wraps the routine in its `RoutineCompleted`
  pub(crate) into_completed: fn(Box<dyn Event>) -> EventBox,
}

impl Debug for RoutineStart {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} from {}", self.name, self.origin.id)
  }
}
