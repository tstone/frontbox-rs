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
//! Regardless of competitive or cooperative play, the turn flow is as follows:
//! 1. A player's turn begins with the `PlayerTurnBeginning` event, and the game manager asks the `BallManager` for a ball. This is a good time to trigger any "start of turn" mechanics, like music and the display.
//! 2. When the ball enters play (`BallEnteredPlay`), or a `ball_in_play_switches` switch is hit, whichever comes first, `PlayerTurnActive` is emitted.
//! 3. When the last ball in play leaves play (`BallsExitedPlay` with none left in play), the `PlayerTurnEnding` event is emitted, signaling the end of the current player's turn. This is a good time for modes to clean up.
//! 4. The `TurnEnding` routine runs: each participant (bonus, shoot again, etc.) takes its turn at the end-of-ball display, animations, etc.
//! 5. Once the last participant completes its step, the game manager advances to the next turn, or ends the game after the last one.
//!
//! The end-of-turn sequence is defined in the app:
//!
//! ```rust,ignore
//! app.routine::<TurnEnding>(routine![BonusPresenter, ShootAgainPresenter]);
//! ```
//!
//! With no `TurnEnding` routine defined, the turn advances right away.
//!
//! ## Required Systems
//!
//! To receive the benefits of this crate, these systems must be added to the root of the app:
//! - `GameManager` - Either competitive or cooperative
//! - `BallManager` - e.g. `BallManager::standard`
//! - Game Start -- `FreePlay` or `CreditsPlay`
//!
//! ## Ball Save, Multiball, Extra Balls, and Similar Mechanics
//! `BallSaveSystem` (in the player template) saves drained balls while any system has requested a ball save. A multiball
//! is a mode asking the `BallManager` for more balls, usually with a ball save request alongside, and checking
//! `BallsExitedPlay.in_play` to know when it's over. Other mechanics that extend a player's turn can register an event
//! interrupt on `BallsExitedPlay`; halting it prevents the transition to `PlayerTurnEnding`.

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
