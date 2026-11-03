//! A turn-based game system for Frontbox. This provides systems, events, and commands for managing either player or team turns,
//! points, and similar mechanics.
//!
//! ### On Naming
//! Whereas many machines display the language "3 balls", this implementation refers to these as "turns". During a single turn, multiple
//! balls may be launched into play, or recovered (ball save, extra ball, etc.). The word "turn" here is clearer.
//!
//! Along these lines, mechanics like "extra ball" are really no more than the existing turn being extended. This is different than some manufacturers
//! who grant what is effectively an additional turns to a player when they earn an extra ball. The difference to the player is extending
//! the current turn (receiving the extra ball on drain) versus playing an additional turn at the end of the game.
//!
//! # Overview: Turn Flow
//! Regardless of competitive or cooperative play, every turn has the same lifecycle:
//! - Routine: `TurnStarting` -- before the ball is served
//! - Event: `TurnActive` -- the served ball is in play
//! - Routine: `BallDraining` -- for each drain, to decide whether it stands
//! - Routine: `TurnEnding` -- after the last ball has left play
//!
//! In detail:
//! 1. A player's turn begins with the game manager activating the player's group and starting the `TurnStarting` routine. `RoutineStarted<TurnStarting>` is broadcast first: this is the time for "start of turn" mechanics, like music, and the playfield drivers are turned on.
//! 2. Each participant of the routine (player-up display, skill shot selection, etc.) takes its turn before the ball comes out.
//! 3. Once the last participant completes its step, `RoutineCompleted<TurnStarting>` is broadcast, and the game manager asks the `BallManager` for a ball.
//! 4. When the ball enters play (`BallEnteredPlay`), or a `ball_in_play_switches` switch is hit, whichever comes first, `TurnActive` is emitted. This happens once per turn, unlike `BallEnteredPlay`, which ball management emits for every ball that goes into play.
//! 5. Each time balls leave play (`BallsExitedPlay`), the game manager starts the `BallDraining` routine. Its participants (ball save, an outlane save, an extra ball, etc.) each get the chance to save the drain, and one that does halts the routine. A drain that comes in while another is being decided waits its turn.
//! 6. When a drain that left no ball in play completes without being halted, the game manager starts the `TurnEnding` routine. `RoutineStarted<TurnEnding>` is broadcast first, signaling that play is over: this is the time for modes to clean up, and the playfield drivers are turned off.
//! 7. Each participant of the routine (bonus, shoot again, etc.) takes its turn at the end-of-ball display, animations, etc.
//! 8. Once the last participant completes its step, `RoutineCompleted<TurnEnding>` is broadcast, and the game manager advances to the next turn, or ends the game after the last one.
//!
//! # Overview: Game End
//! After the last turn, or when the game is quit (`quit_game`, e.g. from `quit_game_combo`), the game manager starts the `GameEnding` routine at the root. It carries the final scores and why the game ended (`GameEndReason`).
//! 1. `RoutineStarted<GameEnding>` is broadcast first, and the playfield drivers are turned off.
//! 2. Each participant (match, high-score entry, game-over display, etc.) takes its turn. The game is still started meanwhile, so the start button can't begin a new one.
//! 3. Once the last participant completes its step, the player groups are despawned, the game is no longer started, and `GameEnded` is emitted. Systems that only run outside a game (attract mode, the start button) are inactive during `GameEnding`, so they watch `GameEnded` rather than the routine's events.
//!
//! The routines are defined in the app:
//!
//! ```rust,ignore
//! app.routine::<TurnStarting>(routine![PlayerUpPresenter]);
//! app.routine::<BallDraining>(routine![BallSaveSystem, ExtraBall]);
//! app.routine::<TurnEnding>(routine![BonusPresenter, ShootAgainPresenter]);
//! app.routine::<GameEnding>(routine![MatchPresenter, HighScoreEntry]);
//! ```
//!
//! With no `TurnStarting` routine defined, the ball is served right away; with no `BallDraining` routine, no drain is
//! saved; with no `TurnEnding` routine, the turn advances right away; with no `GameEnding` routine, the game ends right
//! away.
//!
//! ## Required Systems
//!
//! To receive the benefits of this crate, these systems must be added to the root of the app:
//! - `GameManager` - Either competitive or cooperative
//! - `BallManager` - e.g. `BallManager::standard`
//! - Game Start -- `FreePlay` or `CreditsPlay`
//!
//! ## Ball Save, Multiball, Extra Balls, and Similar Mechanics
//! `BallSaveSystem` (in the player template) saves drained balls while any system has requested a ball save. It takes
//! part in `BallDraining`, so the app lists it there. Other mechanics that save a drain or extend a player's turn
//! (outlane saves, extra balls) are `BallDraining` participants too, and the app picks their order.
//!
//! A multiball is a mode asking the `BallManager` for more balls, usually with a ball save request alongside. It knows
//! it's over from `RoutineCompleted<BallDraining>`: a drain completing with `RoutineCompletion::AllSteps` wasn't saved,
//! and its `in_play` is how many balls are left.

pub mod ball_management;
mod ball_save;
pub mod configs;
mod game_management;
mod game_start;
mod high_scores;
pub mod metrics;

pub use ball_management::*;
pub use ball_save::*;
pub use game_management::*;
pub use game_start::*;
pub use high_scores::*;
