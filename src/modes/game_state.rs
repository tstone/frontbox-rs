#[derive(Debug, Clone)]
pub struct GameState {
  pub(crate) started: bool,
  pub(crate) player_count: u8,
  pub(crate) current_player: Option<u8>,
  pub(crate) current_ball: Option<u8>,
}

impl GameState {
  pub fn is_started(&self) -> bool {
    self.started
  }

  pub fn not_running(&self) -> bool {
    !self.started
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

impl GameState {
  pub fn just_started(old: &GameState, new: &GameState) -> bool {
    !old.is_started() && new.is_started()
  }

  pub fn just_ended(old: &GameState, new: &GameState) -> bool {
    old.is_started() && !new.is_started()
  }

  pub fn changed_player(old: &GameState, new: &GameState) -> bool {
    old.current_player() != new.current_player() && new.current_player().is_some()
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
