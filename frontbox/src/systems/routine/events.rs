use std::any::{Any, type_name};

use crate::app::event_box::EventBox;
use crate::prelude::*;

/// Broadcast when the routine for `E` starts, before its first participant is handed the routine. Carries a copy of
/// the routine. Systems that should react automatically, without a place in the routine's order, watch this, e.g. to
/// clean up before end-of-turn displays run.
///
/// ```rust
/// # use frontbox::prelude::*;
/// # #[derive(Serialize, Event, Clone)]
/// # struct TurnEnding { player: u8 }
/// # struct Mode;
/// # impl Mode { fn shutdown(&mut self, ctx: &SystemContext) {} }
/// impl System for Mode {
///   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
///     if event.is::<RoutineStarted<TurnEnding>>() {
///       self.shutdown(ctx);
///     }
///   }
/// }
/// ```
pub struct RoutineStarted<E: Event> {
  pub routine: E,
}

impl<E: Event + Clone> RoutineStarted<E> {
  /// Copies a routine, which must be an `E`. Stored with the routine, since the run loop holds routines without their
  /// types.
  pub(crate) fn from_routine(routine: &dyn Event) -> EventBox {
    let routine = routine
      .downcast_ref::<E>()
      .expect("a routine is the type it was started with");
    EventBox::new(Self {
      routine: routine.clone(),
    })
  }
}

/// Broadcast once the last participant of the routine for `E` is done, handing back the routine as it was started. The
/// origin watches this to carry on, so it doesn't need to hold on to the routine.
///
/// ```rust
/// # use frontbox::prelude::*;
/// # #[derive(Serialize, Event, Clone)]
/// # struct TurnEnding { player: u8 }
/// # struct Game;
/// impl System for Game {
///   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
///     if let Some(done) = event.downcast_ref::<RoutineCompleted<TurnEnding>>() {
///       log::info!("Player {}'s turn is over", done.routine.player);
///     }
///   }
/// }
/// ```
pub struct RoutineCompleted<E: Event> {
  pub routine: E,
}

impl<E: Event> RoutineCompleted<E> {
  /// Wraps a routine, which must be an `E`
  pub(crate) fn from_routine(routine: Box<dyn Event>) -> EventBox {
    let routine = (routine as Box<dyn Any>)
      .downcast::<E>()
      .expect("a routine is the type it was started with");
    EventBox::new(Self { routine: *routine })
  }
}

// by hand: the derive doesn't handle generics
impl<E: Event> Event for RoutineStarted<E> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn to_json(&self) -> Option<serde_json::Value> {
    routine_json::<E>(&self.routine)
  }
}

impl<E: Event> Event for RoutineCompleted<E> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn to_json(&self) -> Option<serde_json::Value> {
    routine_json::<E>(&self.routine)
  }
}

fn routine_json<E: Event>(routine: &E) -> Option<serde_json::Value> {
  Some(serde_json::json!({
    "name": type_name::<E>(),
    "routine": routine.to_json(),
  }))
}
