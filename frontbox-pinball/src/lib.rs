//! A turn-based game system for Frontbox. This provides systems, events, and commands for managing either player or team turns,
//! points, and similar mechanics.
//!
//! ### On Naming
//! Whereas many machines display the language "3 balls", this implementation refers to these as "turns". During a single turn, multiple
//! balls may be launched into play, or recovered (ball save, etc.). The word "turn" here is clearer.
//!
//! An extra ball is another turn for the same player, played right after the one that earned it ("shoot again"). The
//! turn ends as usual (bonus, mode cleanup), and the next `TurnStarting` is the same player's, with
//! `TurnStartReason::ExtraBall` and the same turn number.
//!
//! # Overview: Game Start
//! When the first player is added (e.g. by `FreePlay` or `CreditsPlay`), the game manager starts the game and starts the `GameStarting` routine at the root.
//! 1. `RoutineStarted<GameStarting>` is broadcast first. The game is started, but no turn has begun and the playfield drivers are still off.
//! 2. Each participant (game-start sound, start-of-game display, etc.) takes its turn. More players can still be added meanwhile.
//! 3. Once the last participant completes its step, the first turn begins with `TurnStarting`.
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
//! 2. Each participant of the routine (player-up or shoot-again display, skill shot selection, etc.) takes its turn before the ball comes out.
//! 3. Once the last participant completes its step, `RoutineCompleted<TurnStarting>` is broadcast, and the game manager asks the `BallManager` for a ball.
//! 4. When the ball enters play (`BallEnteredPlay`), or a `ball_in_play_switches` switch is hit, whichever comes first, `TurnActive` is emitted. This happens once per turn, unlike `BallEnteredPlay`, which ball management emits for every ball that goes into play.
//! 5. Each time balls leave play (`BallsExitedPlay`), the game manager starts the `BallDraining` routine. Its participants (ball save, an outlane save, etc.) each get the chance to save the drain, and one that does halts the routine. A drain that comes in while another is being decided waits its turn.
//! 6. When a drain that left no ball in play completes without being halted, the game manager starts the `TurnEnding` routine. `RoutineStarted<TurnEnding>` is broadcast first, signaling that play is over: this is the time for modes to clean up, and the playfield drivers are turned off.
//! 7. Each participant of the routine (bonus, etc.) takes its turn at the end-of-ball display, animations, etc.
//! 8. Once the last participant completes its step, `RoutineCompleted<TurnEnding>` is broadcast, and the game manager advances to the next turn, or ends the game after the last one. If the player has an extra ball, they use it, and the next turn is theirs again.
//!
//! # Overview: Game End
//! After the last turn, or when the game is quit (`quit_game`, e.g. from `quit_game_combo`), the game manager starts the `GameEnding` routine at the root. It carries the final scores and why the game ended (`GameEndReason`).
//! 1. `RoutineStarted<GameEnding>` is broadcast first, and the playfield drivers are turned off.
//! 2. Each participant (match, high-score entry, game-over display, etc.) takes its turn. The game is still started meanwhile, so the start button can't begin a new one.
//! 3. Once the last participant completes its step, `RoutineCompleted<GameEnding>` goes to the game manager first: the player groups are despawned, and `GamePhase` goes back to `None`. Only then does everyone else hear it, so systems that only run outside a game (attract mode, the start button) are active again by then, and can take the final scores from it.
//!
//! The routines are defined in the app:
//!
//! ```rust,ignore
//! app.routine::<GameStarting>(routine![GameStartPresenter]);
//! app.routine::<TurnStarting>(routine![PlayerUpPresenter]);
//! app.routine::<BallDraining>(routine![BallSaveSystem]);
//! app.routine::<TurnEnding>(routine![BonusPresenter]);
//! app.routine::<GameEnding>(routine![MatchPresenter, HighScoreEntry]);
//! ```
//!
//! With no `GameStarting` routine defined, the first turn begins right away; with no `TurnStarting` routine, the ball
//! is served right away; with no `BallDraining` routine, no drain is
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
//! part in `BallDraining`, so the app lists it there. Other mechanics that save a drain (outlane saves) are
//! `BallDraining` participants too, and the app picks their order.
//!
//! Extra balls are kept by the game manager, per player. Any system grants one with `ctx.grant_extra_ball()`
//! (`GameManagementExt`), up to the `MAX_EXTRA_BALLS` operator config's limit per game; it returns false when the
//! limit is reached, so the caller can award something else.
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
