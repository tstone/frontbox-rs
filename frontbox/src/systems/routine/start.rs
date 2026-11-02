use std::any::TypeId;
use std::fmt::Debug;

use crate::prelude::*;

/// A request to start a routine, sent from a context to the run loop
pub struct RoutineStart {
  pub(crate) origin: SystemHandle,
  pub(crate) type_id: TypeId,
  pub(crate) name: &'static str,
  pub(crate) payload: Box<dyn Event>,
  /// Wraps the payload in its `RoutineComplete`. Chosen here, while the payload's type is still known
  pub(crate) into_complete: fn(Box<dyn Event>) -> Box<dyn Event>,
}

impl Debug for RoutineStart {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} from {}", self.name, self.origin.id)
  }
}
