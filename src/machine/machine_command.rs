use crossterm::event::Event;

use crate::prelude::*;
use crate::protocol::EventResponse;

pub enum MachineCommand {
  // game management
  StartGame,
  EndGame,
  AddPlayer,
  AdvancePlayer,

  // stack management
  PushRuntime(Box<dyn FnOnce() -> Box<dyn Runtime> + Send>),
  PopRuntime,
  PushScene(Scene),
  PopScene,
  AddSystem(Box<dyn System>),
  ReplaceSystem(u64, Box<dyn System>),
  TerminateSystem(u64),

  // hardware
  ConfigureDriver(&'static str, DriverConfig),
  TriggerDriver(&'static str, DriverTriggerControlMode, Option<Duration>),
  HardwareEvent(EventResponse),
  Key(Event),

  // timers
  ClearTimer(u64, &'static str),
  SetTimer(u64, &'static str, Duration, TimerMode),
  SystemTick,
  WatchdogTick,

  // other
  StoreWrite(Box<dyn FnOnce(&mut Store) + Send>),
  SetConfigValue(&'static str, ConfigValue),
  Shutdown,
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
      Self::TriggerDriver(name, mode, delay) => {
        write!(f, "TriggerDriver({:?}, {:?}, {:?})", name, mode, delay)
      }
      Self::StoreWrite(_) => write!(f, "StoreWrite(..)"),
      Self::SetTimer(id, name, duration, mode) => {
        write!(
          f,
          "SetTimer({}, {:?}, {:?}, {:?})",
          id, name, duration, mode
        )
      }
      Self::ClearTimer(id, timer_name) => write!(f, "ClearTimer({}, {:?})", id, timer_name),
      Self::SetConfigValue(key, value) => write!(f, "SetConfigValue({}, {:?})", key, value),
      Self::SystemTick => write!(f, "SystemTick"),
      Self::WatchdogTick => write!(f, "WatchdogTick"),
      Self::HardwareEvent(event) => write!(f, "HardwareEvent({:?})", event),
      Self::Key(key_event) => write!(f, "Key({:?})", key_event),
      Self::Shutdown => write!(f, "Shutdown"),
    }
  }
}
