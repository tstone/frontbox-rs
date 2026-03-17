use frontbox::prelude::Storable;
use serde::Serialize;

/// A state indicating if players can be added or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Storable)]
pub enum GameStartState {
  /// The game can be started.
  GameStartable,
  /// Players can be added and the game can be started.
  PlayerAddable,
  /// Players cannot be added and the game cannot be started.
  NotStartable,
}

/// A state indicating the current phase of a player's turn. Will only be present when a game is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Storable)]
pub enum CurrentPlayerTurnState {
  /// The beginning of a player's turn, before the ball is launched.
  Beginning,
  /// The active part of a player's turn, after the ball is launched and before it goes out of play.
  Active,
  /// The end of a player's turn, after the ball goes out of play but before the next turn begins.
  Ending,
}
