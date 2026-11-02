//! # Routines
//!
//! A routine is an ordered list of systems which take turns handling something, such as the displays at the end of a
//! turn. An origin starts it; the framework hands it to each participant in turn, waits for that participant to say it's
//! done, and tells the origin once the last one has finished.
//!
//! Routines are defined once in the app, keyed by a payload type:
//!
//! ```rust,no_run
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct EndOfTurn;
//! # #[derive(Clone)]
//! # struct BonusPresenter;
//! # impl System for BonusPresenter {}
//! # #[derive(Clone)]
//! # struct ShootAgainPresenter;
//! # impl System for ShootAgainPresenter {}
//! # fn example(app: &mut App) {
//! app.routine::<EndOfTurn>(routine![BonusPresenter, ShootAgainPresenter]);
//! # }
//! ```
//!
//! The origin starts the routine with a payload. When the routine finishes, [`RoutineComplete<E>`] is delivered only to
//! the origin, like a cue, and hands the payload back:
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct EndOfTurn { player: u8 }
//! # struct Game;
//! # impl Game { fn advance_turn(&mut self, ctx: &SystemContext) {} }
//! impl System for Game {
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     if event.is::<RoutineComplete<EndOfTurn>>() {
//!       self.advance_turn(ctx);
//!     }
//!   }
//! }
//!
//! # fn example(ctx: &SystemContext) {
//! ctx.start_routine(EndOfTurn { player: 1 });
//! # }
//! ```
//!
//! Each participant is handed the payload in `on_routine`, and calls `complete_routine_step` when its step is done,
//! either right away or later (from a cue, say):
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct EndOfTurn { player: u8 }
//! #[derive(Serialize, Event)]
//! struct BonusCounted;
//!
//! struct BonusPresenter;
//!
//! impl System for BonusPresenter {
//!   fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
//!     if routine.is::<EndOfTurn>() {
//!       // count up the bonus, finishing in 3 seconds
//!       ctx.cue(BonusCounted, Cue::Once(Duration::from_secs(3)));
//!     }
//!   }
//!
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     if event.is::<BonusCounted>() {
//!       ctx.complete_routine_step::<EndOfTurn>();
//!     }
//!   }
//! }
//! ```
//!
//! The payload is never broadcast: only participants see it. If other systems need to know, the origin emits a
//! separate event when it sees fit.
//!
//! ### Rules
//!
//! - **One per type** -- Only one routine of a given type runs at a time. A second start is ignored with a warning.
//! - **Empty or undefined** routines finish immediately.
//! - **Nearest wins** -- Each listed type resolves to one instance, searched for from the origin's group and then the
//!   root, the same as `ctx.get`. A routine meant for a player's systems should be started by a system in that
//!   player's group.
//! - **Inactive or missing participants are skipped.**
//! - **Only the current participant can complete its step** -- Completion is keyed to the caller, so a system that
//!   does work on a participant's behalf reports back, and the participant completes. Late or duplicate completions
//!   are ignored.
//! - **Despawned or deactivated mid-step counts as complete**, so a routine can't get stuck. Despawning and group
//!   (de)activation take effect in queue order; a system whose own `is_active` changes is noticed when the framework
//!   next checks it, as with anything else. A system replaced with `replace_self` is a new system, so the replacement
//!   can't complete the old step.
//! - **Paused while the origin is inactive** -- Like a cue, a routine doesn't move on while its origin is inactive. The
//!   step in progress carries on, but the next participant isn't handed the routine, nor the origin its
//!   `RoutineComplete`, until the origin is active again. A routine whose origin is despawned is abandoned.

mod complete;
mod definition;
mod routines;
mod start;
#[cfg(test)]
mod tests;

pub use complete::RoutineComplete;
pub use definition::RoutineDefinition;
pub(crate) use routines::Routines;
pub use start::RoutineStart;
