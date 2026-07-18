
/// A state indicating the current phase of a player's turn. Will only be present when a game is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
  /// The beginning of a player's turn, before the ball is launched.
  Beginning,
  /// The active part of a player's turn, after the ball is launched and before it goes out of play.
  Active,
  /// The end of a player's turn, after the ball goes out of play but before the next turn begins.
  Ending,
}

