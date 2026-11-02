use std::any::{Any, TypeId, type_name};
use std::fmt::Debug;

use crate::app::event_box::EventBox;
use crate::prelude::*;

pub(crate) type StateValue = dyn Any + Send + Sync;

/// A state's value and who owns it, as held in its group. Also how a context sends a state it set to the run loop.
pub struct StoredState {
  pub(crate) owner: SystemHandle,
  pub(crate) type_id: TypeId,
  pub(crate) name: &'static str,
  pub(crate) value: Box<StateValue>,
  /// The `StateChanged` from one value to another, or `None` if they're equal. A missing value is the default.
  /// Chosen here, while the state's type is still known.
  pub(crate) into_changed: fn(Option<&StateValue>, Option<&StateValue>) -> Option<EventBox>,
}

impl StoredState {
  pub(crate) fn new<S: State>(owner: SystemHandle, value: S) -> Self {
    Self {
      owner,
      type_id: TypeId::of::<S>(),
      name: type_name::<S>(),
      value: Box::new(value),
      into_changed: into_changed::<S>,
    }
  }
}

impl Debug for StoredState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} from {}", self.name, self.owner.id)
  }
}

fn into_changed<S: State>(from: Option<&StateValue>, to: Option<&StateValue>) -> Option<EventBox> {
  let value = |value: Option<&StateValue>| -> S {
    value
      .map(|value| {
        value
          .downcast_ref::<S>()
          .expect("a state is stored as its own type")
          .clone()
      })
      .unwrap_or_default()
  };
  let (from, to) = (value(from), value(to));
  (from != to).then(|| EventBox::new(StateChanged { from, to }))
}
