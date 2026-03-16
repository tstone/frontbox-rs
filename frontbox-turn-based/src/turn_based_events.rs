pub struct GameStarted;
pub struct GameEnded;

pub enum CurrentPlayerTurnState {
  /// The beginning of a player's turn, before the ball is launched.
  Beginning,
  /// The active part of a player's turn, after the ball is launched and before it goes out of play.
  Active,
  /// The end of a player's turn, after the ball goes out of play but before the next turn begins.
  Ending,
}

/// When the current player's turn starts. This happens at the beginning of each "ball".
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
