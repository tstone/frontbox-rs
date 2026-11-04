use frontbox::prelude::*;

/// The current phase of a player's turn, set by the game manager. Read it with `ctx.states.get::<TurnPhase>()`, or watch
/// `StateChanged<TurnPhase>`.
#[derive(State, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
  /// No turn is in progress: no game is started, `GameStarting` is still running, or the game is over.
  #[default]
  None,
  /// The start of a player's turn, while the `TurnStarting` routine runs and the ball is served, before it's in play.
  Starting,
  /// The active part of a player's turn, after the ball is launched and before it goes out of play.
  Active,
  /// The end of a player's turn, after the ball goes out of play but before the next turn begins. Stays through
  /// `GameEnding`.
  Ending,
}
