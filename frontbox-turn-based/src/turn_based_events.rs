pub struct GameStarted;
pub struct GameEnded;

/// When the current player changes
pub struct CurrentPlayerChanged {
  pub current_player: u8,
}

impl CurrentPlayerChanged {
  pub fn new(current_player: u8) -> Self {
    Self { current_player }
  }
}

/// When a player is added to the game
pub struct PlayerAdded;

impl PlayerAdded {
  pub fn new() -> Self {
    Self
  }
}
