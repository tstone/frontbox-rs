//! # Routines
//!
//! A routine is a lifecycle hook that the app defines. Some system decides when the moment comes, such as a turn
//! ending, and starts the routine; the app decides what happens during it by listing systems, in order. The framework
//! hands the routine to each of those participants in turn, waits for each to say it's done, and announces when the
//! last one has completed.
//!
//! A routine is a type, named in the present progressive for what's going on while it runs (`TurnEnding`), so it reads
//! apart from events, which are named in the past tense (`BallSaved`). It's defined once in the app:
//!
//! ```rust,no_run
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event, Clone)]
//! # struct TurnEnding;
//! # #[derive(Clone)]
//! # struct BonusPresenter;
//! # impl System for BonusPresenter {}
//! # #[derive(Clone)]
//! # struct ShootAgainPresenter;
//! # impl System for ShootAgainPresenter {}
//! # fn example(app: &mut App) {
//! app.routine::<TurnEnding>(routine![BonusPresenter, ShootAgainPresenter]);
//! # }
//! ```
//!
//! ## Participant or listener?
//!
//! Each routine has two broadcast events around its steps, so a system can take part in a routine either as a
//! participant or by listening for those events. Routines are only ever configured by the app: the framework never
//! adds a participant to one. That decides which to use:
//!
//! - **A participant**, when its place in the order matters, so the app has to decide it. Taking part is opted into,
//!   like adding a system to the app. Ball save on a ball-draining routine is an example: the app lists it, and
//!   chooses whether it goes before or after, say, an outlane save.
//! - **A listener** on [`RoutineStarted<E>`] or [`RoutineCompleted<E>`], when it should happen automatically and its
//!   order relative to others doesn't matter. The game manager turning off the playfield drivers on
//!   `RoutineStarted<TurnEnding>` is an example, as are modes cleaning up.
//!
//! A library system that needs a place in the order documents which routine it belongs in, and leaves listing it to
//! the app.
//!
//! ## Running a routine
//!
//! The origin starts the routine with a value of its type, which carries whatever participants need to know. From
//! there:
//!
//! 1. [`RoutineStarted<E>`] is broadcast, before the first participant is handed the routine. Anything its listeners
//!    emit lands before the first step.
//! 2. The participants take their steps, in order.
//! 3. [`RoutineCompleted<E>`] is broadcast once the last participant is done. The origin watches this to carry on.
//!
//! Both events carry the routine as it was started.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event, Clone)]
//! # struct TurnEnding { player: u8 }
//! # struct Game;
//! # impl Game { fn advance_turn(&mut self, ctx: &SystemContext) {} }
//! impl System for Game {
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     if event.is::<RoutineCompleted<TurnEnding>>() {
//!       self.advance_turn(ctx);
//!     }
//!   }
//! }
//!
//! # fn example(ctx: &SystemContext) {
//! ctx.start_routine(TurnEnding { player: 1 });
//! # }
//! ```
//!
//! Each participant is handed the routine in `on_routine`, and calls `complete_routine_step` when its step is done,
//! either right away or later (from a cue, say):
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(Serialize, Event, Clone)]
//! # struct TurnEnding { player: u8 }
//! #[derive(Serialize, Event)]
//! struct BonusCounted;
//!
//! struct BonusPresenter;
//!
//! impl System for BonusPresenter {
//!   fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
//!     if routine.is::<TurnEnding>() {
//!       // count up the bonus, done in 3 seconds
//!       ctx.cue(BonusCounted, Cue::Once(Duration::from_secs(3)));
//!     }
//!   }
//!
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     if event.is::<BonusCounted>() {
//!       ctx.complete_routine_step::<TurnEnding>();
//!     }
//!   }
//! }
//! ```
//!
//! ## Rules
//!
//! - **One per type** -- Only one routine of a given type runs at a time. A second start is ignored with a warning, and
//!   broadcasts nothing.
//! - **Empty or undefined** routines complete immediately, still broadcasting both events.
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
//!   step in progress carries on, but the next participant isn't handed the routine, nor `RoutineCompleted`
//!   broadcast, until the origin is active again. A routine whose origin is despawned is abandoned, and never
//!   completes.

mod definition;
mod events;
mod routines;
mod start;
#[cfg(test)]
mod tests;

pub use definition::RoutineDefinition;
pub use events::{RoutineCompleted, RoutineStarted};
pub(crate) use routines::Routines;
pub use start::RoutineStart;
