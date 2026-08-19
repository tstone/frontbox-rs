use frontbox::prelude::Event;

#[derive(serde::Serialize, Event)]
pub struct GameStarted;
#[derive(serde::Serialize, Event)]
pub struct GameEnded {
  pub scores: Vec<(&'static str, u32)>,
}

/// When the current player's turn starts. This happens at the beginning of each "ball" when the ball has been fed to the plunge lane
#[derive(serde::Serialize, Event)]
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

#[derive(serde::Serialize, Event)]
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
#[derive(serde::Serialize, Event)]
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

#[derive(serde::Serialize, Event)]
pub struct PlayerAdded;

/// When a player receives points
#[derive(serde::Serialize, Event)]
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
