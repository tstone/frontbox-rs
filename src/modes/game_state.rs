#[derive(Debug, Clone)]
pub struct GameState {
  started: bool,
  player_count: u8,
  current_player: Option<u8>,
  current_ball: Option<u8>,
}

impl GameState {
  pub fn is_started(&self) -> bool {
    self.started
  }

  pub fn current_player(&self) -> Option<u8> {
    self.current_player
  }

  pub fn current_ball(&self) -> Option<u8> {
    self.current_ball
  }

  pub fn player_count(&self) -> u8 {
    self.player_count
  }
}

impl Default for GameState {
  fn default() -> Self {
    Self {
      started: false,
      player_count: 0,
      current_player: None,
      current_ball: None,
    }
  }
}
