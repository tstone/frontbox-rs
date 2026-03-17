//! A turn-based game system for Frontbox. This provides systems, events, and commands for managing either player or team turns,
//! points, and similar mechanics.
//!
//! ### On Naming
//! Whereas many machines display the language "3 balls", this implementation refers to these as "turns". During a single turn, multiple
//! balls may be launched into play, or recovered (ball save, extra ball, etc.). The word "turn" here is clearer.
//!
//! Along these lines, mechanics like "extra ball" are really no more than the existing turn being extended. This is different than some manufacturers
//! who grant what is effectively an additional turns to a player when they earn an extra ball. The difference to the player is extending
//! the current turn (receviing the extra ball on drain) versus playing an additional turn at the end of the game.
//!
//! # Overview: Turn Flow
//! Regardless of competitive or cooperative play, the turn flow is as follows:
//! 1. A player's turn begins with the `PlayerTurnBeginning` event. This is a good time to trigger any "start of turn" mechanics, like loading the ball into the plunge lane, from the trough.
//! 2. When the ball becomes in play, the `PlayerTurnActive` must be emitted (detected through `ball_in_playe_switches` set on `PlayerSystem`)
//! 3. Both player and team systems listen for a `TroughFull` event, which indicates that the ball has gone out of play. When this happens, the `PlayerTurnEnding` event is emitted, signaling the end of the current player's turn. This is a good time to trigger any "end of turn" mechanics, such as tallying bonus, displaying to the player, etc.
//!
//! The next turn is not automatically advanced. Instead, a custom system must call the `AdvanceTurn` command when the game is ready. This allows all of the end-of-ball display, animations, etc. to complete before continuing.
//!
//! ## Required Systems
//!
//! To receive the benefits of this crate, one of these two systems must be added to the root of the app:
//! - `PlayerSystem` - For competitive games with 1 or more players
//! - `TeamSystem` - For cooperative games with 2 or more teams. The mapping of player number to team is given at the start.
//!
//! ## Provided Context
//!
//! - `CurrentPlayerTurnState` - If present, indicates the state (beginning, active, ending) of the current player. This is updated automatically with the events.
//! - `PlayersGameState` - Present for competitive games, indicates the current player and turn number. This is read-only and updated automatically.
//! - `TeamGameState` - Present for cooperative games, indicates the current team and turn number. This is read-only and updated automatically.
//!
//! ## Ball Save, Extra Balls, and Similar Mechanics
//! For mechanics that can extend a player's turn, such as ball saves or extra balls, this can be implemented by registering an event interrupt on `TroughFull`. Halting the broadcast of that command will also prevent the player/team system from transitioning to PlayerTurnEnding.
//!

mod commands;
mod events;
mod game_start;
mod game_state;
mod individual_player_system;
mod individual_scoring;
mod states;

pub use commands::*;
pub use events::*;
pub use game_start::*;
pub use game_state::*;
pub use individual_player_system::*;
pub use individual_scoring::*;
pub use states::*;
