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

/// When a player receives points
pub struct PointsAdded {
  pub player_index: u8,
  pub points_received: u32,
  pub total_points: u32,
}

impl PointsAdded {
  pub fn new(player_index: u8, points_received: u32, total_points: u32) -> Self {
    Self {
      player_index,
      points_received,
      total_points,
    }
  }
}
