//! A turn-based game system for Frontbox. This provides a systems, events, and commands for managing either player or team turns,
//! points, and similar mechanics.
//!
//! ### On Naming
//! This implementation follows more of the Stern method where a player having "3 balls" actually means they have 3 turns. Because
//! during a single turn, multiple balls may be launched into play, calling it a "ball" is a bit confusing.
//!
//! Mechanics like "extra ball" are really no more than the existing turn being extended. This is different than JJP and others who
//! grant what is effectively an additional turn to a player when they earn an extra ball. Similar notions like "ball save" just mean
//! that "under these conditions a ball entering the drain is NOT counted as end of turn".
//!
//! # Overview: Turn Flow
//! Regardless of competitive or cooperative play, the turn flow is as follows:
//! 1. A player's turn begins with the `PlayerTurnBeginning` event. This represents the phase where the ball is loaded into the plunge lane, but is not yet in play. This is a good time to trigger any "start of turn" mechanics.
//! 2. When the ball becomes in play, the `PlayerTurnActive` must be emitted. This can typically be achieved by either having a plunge lane exit switch or detecting any playfield switch. Because this varies per machine, it is not automatically emitted but must be triggered by a custom system;
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

mod player;
mod turn_based_events;
mod turn_based_game_commands;

pub use player::*;
pub use turn_based_events::*;
pub use turn_based_game_commands::*;
