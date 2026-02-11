use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use crate::prelude::*;
use tokio::sync::mpsc;

pub struct Context<'a> {
  sender: mpsc::UnboundedSender<MachineCommand>,
  runtime_stack: Rc<RefCell<Vec<Box<dyn Runtime>>>>,
  switches: &'a SwitchContext,
  game_state: &'a Option<GameState>,
  current_system_index: Option<usize>,
}

impl<'a> Context<'a> {
  pub fn new(
    sender: mpsc::UnboundedSender<MachineCommand>,
    runtime_stack: Rc<RefCell<Vec<Box<dyn Runtime>>>>,
    switches: &'a SwitchContext,
    game_state: &'a Option<GameState>,
    current_system_index: Option<usize>,
  ) -> Self {
    Self {
      sender,
      runtime_stack,
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

  fn active_store_mut(&self) -> RefMut<Store> {
    RefMut::map(self.runtime_stack.borrow_mut(), |stack| {
      let runtime = stack.last_mut().unwrap();
      runtime.get_current_store_mut()
    })
  }

  pub fn get<T: Default + 'static>(&mut self) -> RefMut<T> {
    RefMut::map(self.active_store_mut(), |store| store.get_mut::<T>())
  }

  pub fn insert<T: Default + 'static>(&mut self, value: T) {
    self.active_store_mut().insert::<T>(value);
  }

  pub fn remove<T: Default + 'static>(&mut self) {
    self.active_store_mut().remove::<T>();
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
    let _ = self
      .sender
      .send(MachineCommand::ReplaceSystem(Box::new(system)));
  }

  pub fn terminate_system(&mut self, system_id: usize) {
    let _ = self.sender.send(MachineCommand::TerminateSystem(system_id));
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

pub(crate) enum MachineCommand {
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
  ReplaceSystem(Box<dyn System>),
  TerminateSystem(usize),

  // hardware control
  ConfigureDriver(&'static str, DriverConfig),
  TriggerDriver(&'static str, DriverTriggerControlMode),
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
      Self::ReplaceSystem(_) => write!(f, "ReplaceSystem(..)"),
      Self::TerminateSystem(id) => write!(f, "TerminateSystem({})", id),
      Self::ConfigureDriver(name, config) => write!(f, "ConfigureDriver({:?}, {:?})", name, config),
      Self::TriggerDriver(name, mode) => write!(f, "TriggerDriver({:?}, {:?})", name, mode),
    }
  }
}
