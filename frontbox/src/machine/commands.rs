use std::time::Duration;
use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;
use crate::prelude::*;
use crate::systems::*;

/// Commands enqueues mutable actions
pub struct Commands {
  machine: mpsc::UnboundedSender<MachineCommand>,
  system_manager: mpsc::UnboundedSender<SystemCommand>,
  listener_id: u64,
  pub store: WriteableStore,
  pub config: WriteableConfig,
}

impl Commands {
  pub fn new(
    machine: mpsc::UnboundedSender<MachineCommand>,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store_manager: mpsc::UnboundedSender<StoreCommand>,
    listener_id: u64,
  ) -> Self {
    let config = WriteableConfig::new(machine.clone());
    Self {
      machine,
      system_manager,
      listener_id,
      store: WriteableStore::new(store_manager),
      config,
    }
  }

  pub fn clone_for_system(&self, listener_id: u64) -> Self {
    Self::new(
      self.machine.clone(),
      self.system_manager.clone(),
      self.store.sender.clone(),
      listener_id,
    )
  }

  pub fn clone_for_manager(
    &self,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store_manager: mpsc::UnboundedSender<StoreCommand>,
  ) -> Self {
    Self::new(
      self.machine.clone(),
      system_manager,
      store_manager,
      self.listener_id,
    )
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

  pub fn transition(&mut self, new_state: impl StorableType + 'static) {
    let _ = self
      .machine
      .send(MachineCommand::StateTransition(Box::new(move |states| {
        states.set(new_state);
      })));
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

#[derive(Clone)]
pub struct WriteableStore {
  sender: mpsc::UnboundedSender<StoreCommand>,
}

impl WriteableStore {
  pub fn new(sender: mpsc::UnboundedSender<StoreCommand>) -> Self {
    Self { sender }
  }

  pub fn write(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
    let _ = self.sender.send(StoreCommand::Write(Box::new(f)));
  }
}

#[derive(Clone)]
pub struct WriteableConfig {
  sender: mpsc::UnboundedSender<MachineCommand>,
}

impl WriteableConfig {
  pub fn new(sender: mpsc::UnboundedSender<MachineCommand>) -> Self {
    Self { sender }
  }

  pub fn set(&mut self, key: &'static str, value: ConfigValue) {
    let _ = self.sender.send(MachineCommand::SetConfigValue(key, value));
  }
}
