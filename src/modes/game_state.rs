use std::collections::HashMap;

use crate::store::Store;

#[derive(Debug)]
pub struct GameState {
  pub(crate) started: bool,
  pub(crate) player_count: u8,
  pub(crate) team_count: u8,
  pub(crate) current_team: u8,
  pub(crate) current_player: u8,
  pub(crate) player_team_map: HashMap<u8, u8>,
  pub(crate) stores: Vec<Store>,
}

impl Default for GameState {
  fn default() -> Self {
    Self {
      started: false,
      player_count: 0,
      team_count: 0,
      current_team: 0,
      current_player: 0,
      player_team_map: HashMap::new(),
      stores: Vec::new(),
    }
  }
}

impl GameState {
  pub fn new(player_count: u8, team_count: u8, player_team_map: HashMap<u8, u8>) -> Self {
    let mut stores = Vec::new();
    for _ in 0..player_count {
      stores.push(Store::new());
    }

    Self {
      started: true,
      player_count,
      team_count,
      current_team: 0,
      current_player: 0,
      player_team_map,
      stores,
    }
  }

  pub fn get<T: Default + 'static>(&mut self) -> &T {
    self.stores[self.current_player as usize].get_state::<T>()
  }

  pub fn get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.stores[self.current_player as usize].get_state_mut::<T>()
  }

  pub fn insert<T: Default + 'static>(&mut self, value: T) {
    self.stores[self.current_player as usize].insert_state::<T>(value);
  }

  pub fn remove<T: Default + 'static>(&mut self) {
    self.stores[self.current_player as usize].remove_state::<T>();
  }

  pub fn player_count(&self) -> u8 {
    self.player_count
  }

  pub fn team_count(&self) -> u8 {
    self.team_count
  }

  pub fn current_team(&self) -> u8 {
    self.current_team
  }

  pub fn current_player(&self) -> u8 {
    self.current_player
  }

  pub fn player_team(&self, player: u8) -> Option<u8> {
    self.player_team_map.get(&player).copied()
  }

  pub fn team_players(&self, team: u8) -> Vec<u8> {
    self
      .player_team_map
      .iter()
      .filter_map(|(&player, &player_team)| {
        if player_team == team {
          Some(player)
        } else {
          None
        }
      })
      .collect()
  }

  // ---

  pub(crate) fn add_player(&mut self, team: Option<u8>) {
    self.player_count += 1;
    let player_index = self.player_count - 1;

    if let Some(team) = team {
      self.player_team_map.insert(player_index, team);
    } else {
      // with no explicit team, assign player to their own team
      self.player_team_map.insert(player_index, player_index);
    }
  }
}
