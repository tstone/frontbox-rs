use frontbox::prelude::Event;

#[derive(serde::Serialize, Event)]
pub struct GameStarted;
#[derive(serde::Serialize, Event, Clone)]
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

/// Routine run once the last ball of a turn has left play. Its participants are the end-of-turn displays and sequences
/// (bonus, shoot again, etc.); the turn advances once the last one completes its step. With no routine defined, the
/// turn advances right away.
///
/// ```rust,ignore
/// app.routine::<TurnEnding>(routine![BonusPresenter, ShootAgainPresenter]);
/// ```
///
/// Other systems watch the routine's events:
/// - `RoutineStarted<TurnEnding>` -- play is over, before any participant runs. Modes clean up here.
/// - `RoutineCompleted<TurnEnding>` -- the turn is over, just before the next one begins or the game ends.
#[derive(serde::Serialize, Event, Clone)]
pub struct TurnEnding {
  pub current_player: u8,
  pub turn: u8,
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
