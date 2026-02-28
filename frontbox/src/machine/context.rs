use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;
use crate::prelude::*;
use crate::systems::*;

/// Context bridges Systems and Districts to the Machine
pub struct Context<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  stores: HashMap<&'static str, StoreContext<'a>>,
  switches: &'a SwitchContext,
  game_state: &'a Option<GameState>,
  config: ConfigContext<'a>,
  pub(crate) listener_id: u64,
  pub(crate) current_district_key: &'static str,
}

// TODO: make an immutable context, no setting timers or adding listeners
impl<'a> Context<'a> {
  pub fn new(
    sender: mpsc::UnboundedSender<MachineCommand>,
    store: &'a HashMap<&'static str, Box<dyn StorageDistrict>>,
    switches: &'a SwitchContext,
    game_state: &'a Option<GameState>,
    config: &'a MachineConfig,
    listener_id: u64,
    current_district_key: &'static str,
  ) -> Self {
    let stores = store
      .iter()
      .map(|(key, district)| {
        (
          *key,
          StoreContext::new(sender.clone(), district.get_current()),
        )
      })
      .collect();

    Self {
      stores,
      config: ConfigContext::new(sender.clone(), config),
      sender,
      switches,
      game_state,
      listener_id,
      current_district_key,
    }
  }

  pub fn config(&self) -> &ConfigContext<'_> {
    &self.config
  }

  /// Gets the store for a district by key
  pub fn keyed_store(&self, key: &'static str) -> Option<&StoreContext<'_>> {
    self.stores.get(key)
  }

  /// Gets the store for the current district
  pub fn store(&self) -> &StoreContext<'_> {
    self.stores.get(self.current_district_key).unwrap()
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
    let _ = self.sender.send(MachineCommand::SetTimer(
      self.current_district_key,
      self.listener_id,
      timer_name,
      duration,
      mode,
    ));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    let _ = self.sender.send(MachineCommand::ClearTimer(
      self.current_district_key,
      self.listener_id,
      timer_name,
    ));
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

  pub fn insert_runtime(&mut self, key: &'static str, runtime: impl District + Send + 'static) {
    let _ = self.sender.send(MachineCommand::InsertDistrict(
      key,
      Box::new(|| Box::new(runtime) as Box<dyn District>),
    ));
  }

  pub fn remove_runtime(&mut self, key: &'static str) {
    let _ = self.sender.send(MachineCommand::RemoveDistrict(key));
  }

  pub fn add_system(&mut self, system: impl System + 'static) {
    let _ = self.sender.send(MachineCommand::AddSystem(
      self.current_district_key,
      Box::new(system),
    ));
  }

  pub fn replace_system(&mut self, system: impl System + 'static) {
    let _ = self.sender.send(MachineCommand::ReplaceSystem(
      self.current_district_key,
      self.listener_id,
      Box::new(system),
    ));
  }

  pub fn terminate_system(&mut self) {
    let _ = self.sender.send(MachineCommand::TerminateSystem(
      self.current_district_key,
      self.listener_id,
    ));
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

  pub fn subscribe<T: FrontboxEvent>(
    &mut self,
    callback: impl Fn(&T, &mut Context) + Send + 'static,
  ) {
    let _ = self.sender.send(MachineCommand::SubscribeEvent(
      TypeId::of::<T>(),
      self.current_district_key,
      self.listener_id,
      Box::new(move |event, ctx| {
        if let Some(typed_event) = event.as_any().downcast_ref::<T>() {
          callback(typed_event, ctx);
        }
      }),
    ));
  }

  pub fn unsubscribe<T: FrontboxEvent>(&mut self) {
    let _ = self.sender.send(MachineCommand::UnsubscribeEvent(
      TypeId::of::<T>(),
      self.listener_id,
    ));
  }

  /// Broadcast an event to all listeners
  pub fn emit(&mut self, event: Box<dyn FrontboxEvent>) {
    let _ = self.sender.send(MachineCommand::EmitEvent(event));
  }

  /// Send an event to specific listener(s) by ID
  pub(crate) fn target(&mut self, targets: HashSet<u64>, event: Box<dyn FrontboxEvent>) {
    let _ = self
      .sender
      .send(MachineCommand::TargetEvent(targets, event));
  }
}

pub struct StoreContext<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  store: &'a Store,
}

impl<'a> StoreContext<'a> {
  pub fn new(sender: mpsc::UnboundedSender<MachineCommand>, store: &'a Store) -> Self {
    Self { sender, store }
  }

  pub fn exists<T: StorableType>(&self) -> bool {
    self.store.get::<T>().is_some()
  }

  pub fn get<T: StorableType>(&self) -> Option<&T> {
    self.store.get::<T>()
  }

  pub fn with(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
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
