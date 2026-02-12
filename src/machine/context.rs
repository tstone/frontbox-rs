use crate::prelude::*;
use tokio::sync::mpsc;

pub struct Context<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  store: Option<&'a Store>,
  switches: &'a SwitchContext,
  game_state: &'a Option<GameState>,
  current_system_index: Option<usize>,
}

impl<'a> Context<'a> {
  pub fn new(
    sender: mpsc::UnboundedSender<MachineCommand>,
    store: Option<&'a Store>,
    switches: &'a SwitchContext,
    game_state: &'a Option<GameState>,
    current_system_index: Option<usize>,
  ) -> Self {
    Self {
      sender,
      store,
      switches,
      game_state,
      current_system_index,
    }
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }

  pub fn is_game_started(&self) -> bool {
    self.game_state.is_some()
  }

  pub fn active_player(&self) -> Option<u8> {
    if let Some(game_state) = &self.game_state {
      Some(game_state.active_player)
    } else {
      None
    }
  }

  pub fn get<T: Default + 'static>(&self) -> Option<&T> {
    self.store.and_then(|store| store.get::<T>())
  }

  pub fn with_store<T: Default>(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
    if self.store.is_none() {
      log::warn!("No store is available in the current context");
      return;
    }

    let _ = self.sender.send(MachineCommand::StoreWrite(Box::new(f)));
  }

  pub fn start_game(&mut self) {
    let _ = self.sender.send(MachineCommand::StartGame);
  }

  pub fn end_game(&mut self) {
    let _ = self.sender.send(MachineCommand::EndGame);
  }

  pub fn add_player(&mut self) {
    let _ = self.sender.send(MachineCommand::AddPlayer);
  }

  pub fn advance_player(&mut self) {
    let _ = self.sender.send(MachineCommand::AdvancePlayer);
  }

  pub fn push_runtime(&mut self, runtime: impl Runtime + 'static) {
    let _ = self
      .sender
      .send(MachineCommand::PushRuntime(Box::new(runtime)));
  }

  pub fn pop_runtime(&mut self) {
    let _ = self.sender.send(MachineCommand::PopRuntime);
  }

  pub fn push_scene(&mut self, scene: Scene) {
    let _ = self.sender.send(MachineCommand::PushScene(scene));
  }

  pub fn pop_scene(&mut self) {
    let _ = self.sender.send(MachineCommand::PopScene);
  }

  pub fn add_system(&mut self, system: impl System + 'static) {
    let _ = self
      .sender
      .send(MachineCommand::AddSystem(Box::new(system)));
  }

  pub fn replace_system(&mut self, system: impl System + 'static) {
    if let Some(system_id) = self.current_system_index {
      let _ = self
        .sender
        .send(MachineCommand::ReplaceSystem(system_id, Box::new(system)));
    } else {
      log::warn!("No current system index available for replacement");
    }
  }

  pub fn terminate_system(&mut self) {
    if let Some(system_id) = self.current_system_index {
      let _ = self.sender.send(MachineCommand::TerminateSystem(system_id));
    } else {
      log::warn!("No current system index available for termination");
    }
  }

  pub fn configure_driver(&mut self, driver_name: &'static str, config: DriverConfig) {
    let _ = self
      .sender
      .send(MachineCommand::ConfigureDriver(driver_name, config));
  }

  pub fn trigger_driver(&mut self, driver_name: &'static str, mode: DriverTriggerControlMode) {
    let _ = self
      .sender
      .send(MachineCommand::TriggerDriver(driver_name, mode));
  }
}

pub enum MachineCommand {
  // game management
  StartGame,
  EndGame,
  AddPlayer,
  AdvancePlayer,

  // stack management
  PushRuntime(Box<dyn Runtime>),
  PopRuntime,
  PushScene(Scene),
  PopScene,
  AddSystem(Box<dyn System>),
  ReplaceSystem(usize, Box<dyn System>),
  TerminateSystem(usize),

  // hardware control
  ConfigureDriver(&'static str, DriverConfig),
  TriggerDriver(&'static str, DriverTriggerControlMode),

  // store
  StoreWrite(Box<dyn FnOnce(&mut Store) + Send>),
}

impl std::fmt::Debug for MachineCommand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::StartGame => write!(f, "StartGame"),
      Self::EndGame => write!(f, "EndGame"),
      Self::AddPlayer => write!(f, "AddPlayer"),
      Self::AdvancePlayer => write!(f, "AdvancePlayer"),
      Self::PushRuntime(_) => write!(f, "PushRuntime(..)"),
      Self::PopRuntime => write!(f, "PopRuntime"),
      Self::PushScene(_) => write!(f, "PushScene(..)"),
      Self::PopScene => write!(f, "PopScene"),
      Self::AddSystem(_) => write!(f, "AddSystem(..)"),
      Self::ReplaceSystem(id, _) => write!(f, "ReplaceSystem({}, ..)", id),
      Self::TerminateSystem(id) => write!(f, "TerminateSystem({})", id),
      Self::ConfigureDriver(name, config) => write!(f, "ConfigureDriver({:?}, {:?})", name, config),
      Self::TriggerDriver(name, mode) => write!(f, "TriggerDriver({:?}, {:?})", name, mode),
      Self::StoreWrite(_) => write!(f, "StoreWrite(..)"),
    }
  }
}
