use crate::prelude::*;

/// Context bridges Systems to the Machine
pub struct Context<'a> {
  pub config: ReadonlyConfig<'a>,
  pub game: Option<ReadonlyGameState<'a>>,
  pub states: &'a States,
  pub store: ReadonlyStore<'a>,
  switches: &'a SwitchContext,
}

impl<'a> Context<'a> {
  pub fn new(
    config: &'a MachineConfig,
    game_state: &'a Option<GameState>,
    states: &'a States,
    store: &'a Store,
    switches: &'a SwitchContext,
  ) -> Self {
    Self {
      config: ReadonlyConfig::new(config),
      game: game_state.as_ref().map(|gs| ReadonlyGameState::new(gs)),
      states,
      store: ReadonlyStore::new(store),
      switches,
    }
  }

  pub fn clone_for_manager(&self, states: &'a States, store: &'a Store) -> Self {
    Self {
      config: ReadonlyConfig::new(self.config.config),
      game: self
        .game
        .as_ref()
        .map(|gs| ReadonlyGameState::new(gs.game_state)),
      states,
      store: ReadonlyStore::new(store),
      switches: self.switches,
    }
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }
}

pub struct ReadonlyGameState<'a> {
  game_state: &'a GameState,
}

impl<'a> ReadonlyGameState<'a> {
  pub fn new(game_state: &'a GameState) -> Self {
    Self { game_state }
  }

  pub fn active_player(&self) -> u8 {
    self.game_state.active_player
  }

  pub fn player_count(&self) -> u8 {
    self.game_state.player_count
  }
}

pub struct ReadonlyStore<'a> {
  store: &'a Store,
}

impl<'a> ReadonlyStore<'a> {
  pub fn new(store: &'a Store) -> Self {
    Self { store }
  }

  pub fn exists<T: StorableType>(&self) -> bool {
    self.store.get::<T>().is_some()
  }

  pub fn get<T: StorableType>(&self) -> Option<&T> {
    self.store.get::<T>()
  }
}

pub struct ReadonlyConfig<'a> {
  pub(crate) config: &'a MachineConfig,
}

impl<'a> ReadonlyConfig<'a> {
  pub fn new(config: &'a MachineConfig) -> Self {
    Self { config }
  }

  pub fn get(&self, key: &'static str) -> Option<ConfigValue> {
    self.config.get_value(key)
  }
}
