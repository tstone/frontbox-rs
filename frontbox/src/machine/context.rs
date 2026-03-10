use std::time::Duration;
use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;
use crate::prelude::*;
use crate::systems::*;

/// Context bridges Systems and Districts to the Machine
pub struct Context<'a> {
  machine: mpsc::UnboundedSender<MachineCommand>,
  switches: &'a SwitchContext,
  game_state: &'a Option<GameState>,
  config: ConfigContext<'a>,
  system_manager: mpsc::UnboundedSender<SystemCommand>,
  store: StoreContext<'a>,
  listener_id: u64,
}

impl<'a> Context<'a> {
  pub fn new(
    machine: mpsc::UnboundedSender<MachineCommand>,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store: StoreContext<'a>,
    switches: &'a SwitchContext,
    game_state: &'a Option<GameState>,
    config: &'a MachineConfig,
    listener_id: u64,
  ) -> Self {
    Self {
      store,
      config: ConfigContext::new(machine.clone(), config),
      machine,
      system_manager,
      switches,
      game_state,
      listener_id,
    }
  }

  pub fn clone_for_system(&self, listener_id: u64) -> Self {
    Self {
      store: self.store.clone(),
      config: self.config.clone(),
      machine: self.machine.clone(),
      system_manager: self.system_manager.clone(),
      switches: self.switches,
      game_state: self.game_state,
      listener_id,
    }
  }

  /// Creates a new Context for a system manager
  pub fn clone_for_manager(
    &self,
    listener_id: u64,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store: StoreContext<'a>,
  ) -> Self {
    Self {
      store,
      config: self.config.clone(),
      machine: self.machine.clone(),
      system_manager,
      switches: self.switches,
      game_state: self.game_state,
      listener_id,
    }
  }

  pub fn config(&self) -> &ConfigContext<'_> {
    &self.config
  }

  /// Gets the store for the current district
  pub fn store(&self) -> &StoreContext<'_> {
    &self.store
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

  pub fn set_timer(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    let _ = self.system_manager.send(SystemCommand::SetTimer(
      self.listener_id,
      timer_name,
      duration,
      mode,
    ));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    let _ = self
      .system_manager
      .send(SystemCommand::ClearTimer(self.listener_id, timer_name));
  }

  pub fn start_game(&mut self) {
    let _ = self.machine.send(MachineCommand::StartGame);
  }

  pub fn end_game(&mut self) {
    let _ = self.machine.send(MachineCommand::EndGame);
  }

  pub fn add_player(&mut self) {
    let _ = self.machine.send(MachineCommand::AddPlayer);
  }

  pub fn advance_player(&mut self) {
    let _ = self.machine.send(MachineCommand::AdvancePlayer);
  }

  pub fn spawn_system(&mut self, system: impl System + 'static) {
    let _ = self
      .system_manager
      .send(SystemCommand::SpawnSystem(Box::new(system)));
  }

  pub fn replace_system(&mut self, system: impl System + 'static) {
    let _ = self.system_manager.send(SystemCommand::ReplaceSystem(
      self.listener_id,
      Box::new(system),
    ));
  }

  pub fn despawn_system(&mut self) {
    let _ = self
      .system_manager
      .send(SystemCommand::DespawnSystem(self.listener_id));
  }

  pub fn configure_driver(&mut self, driver_name: &'static str, config: DriverConfig) {
    let _ = self
      .machine
      .send(MachineCommand::ConfigureDriver(driver_name, config));
  }

  pub fn trigger_driver(&mut self, driver_name: &'static str, mode: DriverTriggerControlMode) {
    let _ = self
      .machine
      .send(MachineCommand::TriggerDriver(driver_name, mode, None));
  }

  /// Triggers a driver after the given delay time has elapsed
  pub fn trigger_delayed_driver(
    &mut self,
    driver_name: &'static str,
    mode: DriverTriggerControlMode,
    delay: Duration,
  ) {
    let _ = self.machine.send(MachineCommand::TriggerDriver(
      driver_name,
      mode,
      Some(delay),
    ));
  }

  /// Broadcast an event to all listeners
  pub fn emit(&mut self, event: Box<dyn FrontboxEvent>) {
    let _ = self.machine.send(MachineCommand::EmitEvent(event));
  }
}

pub struct StoreContext<'a> {
  sender: mpsc::UnboundedSender<StoreCommand>,
  store: &'a Store,
}

impl<'a> StoreContext<'a> {
  pub fn new(sender: mpsc::UnboundedSender<StoreCommand>, store: &'a Store) -> Self {
    Self { sender, store }
  }

  pub fn exists<T: StorableType>(&self) -> bool {
    self.store.get::<T>().is_some()
  }

  pub fn get<T: StorableType>(&self) -> Option<&T> {
    self.store.get::<T>()
  }

  pub fn with(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
    let _ = self.sender.send(StoreCommand::Write(Box::new(f)));
  }

  pub fn clone(&self) -> Self {
    Self {
      sender: self.sender.clone(),
      store: self.store,
    }
  }
}

pub struct ConfigContext<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  config: &'a MachineConfig,
}

impl<'a> ConfigContext<'a> {
  pub fn new(sender: mpsc::UnboundedSender<MachineCommand>, config: &'a MachineConfig) -> Self {
    Self { sender, config }
  }

  pub fn get(&self, key: &'static str) -> Option<ConfigValue> {
    self.config.get_value(key)
  }

  pub fn set(&mut self, key: &'static str, value: ConfigValue) {
    let _ = self.sender.send(MachineCommand::SetConfigValue(key, value));
  }

  pub fn clone(&self) -> Self {
    Self {
      sender: self.sender.clone(),
      config: self.config,
    }
  }
}
