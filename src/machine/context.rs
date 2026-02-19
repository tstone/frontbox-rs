use std::time::Duration;
use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;
use crate::prelude::*;

pub struct Context<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  store: StoreContext<'a>,
  switches: &'a SwitchContext,
  game_state: &'a Option<GameState>,
  config: ConfigContext<'a>,
  current_system_index: Option<u64>,
}

impl<'a> Context<'a> {
  pub fn new(
    sender: mpsc::UnboundedSender<MachineCommand>,
    store: Option<&'a Store>,
    switches: &'a SwitchContext,
    game_state: &'a Option<GameState>,
    config: &'a MachineConfig,
    current_system_index: Option<u64>,
  ) -> Self {
    Self {
      store: StoreContext::new(sender.clone(), store),
      config: ConfigContext::new(sender.clone(), config),
      sender,
      switches,
      game_state,
      current_system_index,
    }
  }

  pub fn config(&self) -> &ConfigContext<'_> {
    &self.config
  }

  /// Runtime-specific storage of arbitrary data
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
    if let Some(system_id) = self.current_system_index {
      let _ = self.sender.send(MachineCommand::SetTimer(
        system_id, timer_name, duration, mode,
      ));
    } else {
      log::warn!("No current system to set timer for");
    }
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    if let Some(system_id) = self.current_system_index {
      let _ = self
        .sender
        .send(MachineCommand::ClearTimer(system_id, timer_name));
    } else {
      log::warn!("No current system to clear timer for");
    }
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

  pub fn push_runtime(&mut self, runtime: impl Runtime + Send + 'static) {
    let _ = self.sender.send(MachineCommand::PushRuntime(Box::new(|| {
      Box::new(runtime) as Box<dyn Runtime>
    })));
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
      .send(MachineCommand::TriggerDriver(driver_name, mode, None));
  }

  /// Triggers a driver after the given delay time has elapsed
  pub fn trigger_delayed_driver(
    &mut self,
    driver_name: &'static str,
    mode: DriverTriggerControlMode,
    delay: Duration,
  ) {
    let _ = self.sender.send(MachineCommand::TriggerDriver(
      driver_name,
      mode,
      Some(delay),
    ));
  }
}

pub struct StoreContext<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  store: Option<&'a Store>,
}

impl<'a> StoreContext<'a> {
  pub fn new(sender: mpsc::UnboundedSender<MachineCommand>, store: Option<&'a Store>) -> Self {
    Self { sender, store }
  }

  pub fn is_present<T: Default + 'static>(&self) -> bool {
    self.store.is_some()
  }

  pub fn exists<T: Default + 'static>(&self) -> bool {
    self.store.and_then(|store| store.get::<T>()).is_some()
  }

  pub fn get<T: Default + 'static>(&self) -> Option<&T> {
    self.store.and_then(|store| store.get::<T>())
  }

  pub fn with(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
    if self.store.is_none() {
      log::warn!("No store is available in the current context");
      return;
    }

    let _ = self.sender.send(MachineCommand::StoreWrite(Box::new(f)));
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
}
