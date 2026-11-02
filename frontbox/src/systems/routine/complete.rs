use std::any::{Any, type_name};

use crate::prelude::*;

/// Delivered only to a routine's origin, once the last participant of the routine for `E` is done. Carries the payload
/// the routine was started with, so the origin doesn't need to hold on to it.
///
/// ```rust
/// # use frontbox::prelude::*;
/// # #[derive(Serialize, Event)]
/// # struct EndOfTurn { player: u8 }
/// # struct Game;
/// impl System for Game {
///   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
///     if let Some(done) = event.downcast_ref::<RoutineComplete<EndOfTurn>>() {
///       log::info!("Player {}'s turn is over", done.payload.player);
///     }
///   }
/// }
/// ```
pub struct RoutineComplete<E: Event> {
  pub payload: E,
}

impl<E: Event> RoutineComplete<E> {
  /// Wraps a routine's payload, which must be an `E`. Stored with the routine, since the run loop holds payloads
  /// without their types.
  pub(crate) fn from_payload(payload: Box<dyn Event>) -> Box<dyn Event> {
    let payload = (payload as Box<dyn Any>)
      .downcast::<E>()
      .expect("a routine's payload is the type it was started with");
    Box::new(Self { payload: *payload })
  }
}

// by hand: the derive doesn't handle generics
impl<E: Event> Event for RoutineComplete<E> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn to_json(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
      "routine": type_name::<E>(),
      "payload": self.payload.to_json(),
    }))
  }
}
