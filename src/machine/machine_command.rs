use crossterm::event::Event;

use crate::prelude::*;
use crate::protocol::EventResponse;

pub enum MachineCommand {
  // game management
  StartGame,
  EndGame,
  AddPlayer,
  AdvancePlayer,

  // system management
  InsertDistrict(&'static str, Box<dyn FnOnce() -> Box<dyn District> + Send>),
  RemoveDistrict(&'static str),
  AddSystem(&'static str, Box<dyn System>),
  ReplaceSystem(&'static str, u64, Box<dyn System>),
  TerminateSystem(&'static str, u64),

  // hardware
  ConfigureDriver(&'static str, DriverConfig),
  TriggerDriver(&'static str, DriverTriggerControlMode, Option<Duration>),
  HardwareEvent(EventResponse),
  Key(Event),
  ResetExpansionNetwork,

  // timers
  ClearTimer(&'static str, u64, &'static str),
  SetTimer(&'static str, u64, &'static str, Duration, TimerMode),
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
      Self::InsertDistrict(key, _) => write!(f, "InsertDistrict({})", key),
      Self::RemoveDistrict(key) => write!(f, "RemoveDistrict({})", key),
      Self::AddSystem(key, _) => write!(f, "AddSystem({})", key),
      Self::ReplaceSystem(district_key, system_id, _) => {
        write!(f, "ReplaceSystem({}, {}, ..)", district_key, system_id)
      }
      Self::TerminateSystem(district_key, system_id) => {
        write!(f, "TerminateSystem({}, {})", district_key, system_id)
      }
      Self::ConfigureDriver(name, config) => write!(f, "ConfigureDriver({:?}, {:?})", name, config),
      Self::TriggerDriver(name, mode, delay) => {
        write!(f, "TriggerDriver({:?}, {:?}, {:?})", name, mode, delay)
      }
      Self::StoreWrite(_) => write!(f, "StoreWrite(..)"),
      Self::SetTimer(district_key, system_id, timer_name, duration, mode) => {
        write!(
          f,
          "SetTimer({}, {}, {:?}, {:?}, {:?})",
          district_key, system_id, timer_name, duration, mode
        )
      }
      Self::ClearTimer(district_key, system_id, timer_name) => write!(
        f,
        "ClearTimer({}, {}, {:?})",
        district_key, system_id, timer_name
      ),
      Self::SetConfigValue(key, value) => write!(f, "SetConfigValue({}, {:?})", key, value),
      Self::SystemTick => write!(f, "SystemTick"),
      Self::WatchdogTick => write!(f, "WatchdogTick"),
      Self::HardwareEvent(event) => write!(f, "HardwareEvent({:?})", event),
      Self::Key(key_event) => write!(f, "Key({:?})", key_event),
      Self::Shutdown => write!(f, "Shutdown"),
      Self::ResetExpansionNetwork => write!(f, "ResetExpansionNetwork"),
    }
  }
}
