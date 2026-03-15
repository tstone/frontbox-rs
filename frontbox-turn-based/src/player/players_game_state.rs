use frontbox::prelude::{Serialize, Storable};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Storable)]
pub struct PlayersGameState {
  pub(crate) player_count: u8,
  pub(crate) max_players: u8,
  pub(crate) current_player: u8,
}

impl PlayersGameState {
  pub fn new(max_players: u8) -> Self {
    Self {
      player_count: 0,
      max_players,
      current_player: 0,
    }
  }

  pub fn player_count(&self) -> u8 {
    self.player_count
  }

  pub fn max_players(&self) -> u8 {
    self.max_players
  }

  /// Index of the current player
  pub fn current_player(&self) -> u8 {
    self.current_player
  }
}
