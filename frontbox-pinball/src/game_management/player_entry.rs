use frontbox::prelude::*;

/// Whether a player can be added, set by the game manager: by pressing start, say. Read it with
/// `ctx.states.get::<PlayerEntry>()`, or watch `StateChanged<PlayerEntry>`.
#[derive(State, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEntry {
  /// No player can be added: the first turn is over, the game is full, or it's ending. Also the case with no game
  /// manager running.
  #[default]
  Closed,
  /// A player can be added. With no game started, adding one starts a game; otherwise they join the game in progress,
  /// up until the first turn ends.
  Open,
}
