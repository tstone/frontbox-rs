use frontbox_core::prelude::*;

/// The current phase of the game, set by the game manager. Read it with `ctx.states.get::<GamePhase>()`, or watch
/// `StateChanged<GamePhase>`.
#[derive(State, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
  /// No game is started.
  #[default]
  None,
  /// The game has started, and the `GameStarting` routine runs. No turn has begun yet.
  Starting,
  /// Turns are being played, from the first turn's `TurnStarting` until play is over.
  Active,
  /// Play is over, and the `GameEnding` routine runs. The game is still started.
  Ending,
}
