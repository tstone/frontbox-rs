pub struct GameStarted;
pub struct GameEnded;

/// When the current player's turn starts. This happens at the beginning of each "ball" when the ball has been fed to the plunge lane
pub struct PlayerTurnBeginning {
  pub current_player: u8,
  pub turn: u8,
}

impl PlayerTurnBeginning {
  pub fn new(current_player: u8, turn: u8) -> Self {
    Self {
      current_player,
      turn,
    }
  }
}

pub struct PlayerTurnActive {
  pub current_player: u8,
  pub turn: u8,
}

impl PlayerTurnActive {
  pub fn new(current_player: u8, turn: u8) -> Self {
    Self {
      current_player,
      turn,
    }
  }
}

/// Emitted when the ball goes out of play and is in the trough. This would be the time to render bonus scores, show the player ball end information, etc.
pub struct PlayerTurnEnding {
  pub current_player: u8,
  pub turn: u8,
}

impl PlayerTurnEnding {
  pub fn new(current_player: u8, turn: u8) -> Self {
    Self {
      current_player,
      turn,
    }
  }
}

pub struct PlayerAdded;

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
