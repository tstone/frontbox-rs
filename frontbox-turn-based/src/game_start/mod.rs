//! Game start systems and commands for turn-based games.
//!
//! Game start systems are responsible for checking GameStartState to verify if the game can be started or player can be added.

mod free_play;
mod startable_flasher;

pub use free_play::*;
pub use startable_flasher::*;
