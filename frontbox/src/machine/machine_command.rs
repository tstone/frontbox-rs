use crossterm::event::Event;

use crate::machine::event::FrontboxEvent;
use crate::prelude::*;
use fast_protocol::EventResponse;

pub enum MachineCommand {
  // game management
  StartGame,
  EndGame,
  AddPlayer,
  AdvancePlayer,

  // hardware
  ConfigureDriver(&'static str, Box<dyn DriverMode + Send>),
  TriggerDriver(&'static str, DriverTriggerControlMode, Option<Duration>),
  TriggerDriverGroup(&'static str, DriverTriggerControlMode, Option<Duration>),
  HardwareEvent(EventResponse),
  Key(Event),
  ResetExpansionNetwork,

  // timers
  SystemTick,
  WatchdogTick,

  // other
  StateTransition(Box<dyn FnOnce(&mut States) + Send>),
  EmitEvent(Box<dyn FrontboxEvent>),
  SetConfigValue(&'static str, ConfigValue),
  Shutdown,
}

impl std::fmt::Debug for MachineCommand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::EmitEvent(event) => write!(f, "EmitEvent({:?})", event),
      Self::StartGame => write!(f, "StartGame"),
      Self::EndGame => write!(f, "EndGame"),
      Self::AddPlayer => write!(f, "AddPlayer"),
      Self::AdvancePlayer => write!(f, "AdvancePlayer"),
      Self::ConfigureDriver(name, _mode) => write!(f, "ConfigureDriver({:?}, ...)", name),
      Self::TriggerDriver(name, mode, delay) => {
        write!(f, "TriggerDriver({:?}, {:?}, {:?})", name, mode, delay)
      }
      Self::TriggerDriverGroup(name, mode, delay) => {
        write!(f, "TriggerDriverGroup({:?}, {:?}, {:?})", name, mode, delay)
      }
      Self::SetConfigValue(key, value) => write!(f, "SetConfigValue({}, {:?})", key, value),
      Self::SystemTick => write!(f, "SystemTick"),
      Self::WatchdogTick => write!(f, "WatchdogTick"),
      Self::HardwareEvent(event) => write!(f, "HardwareEvent({:?})", event),
      Self::Key(key_event) => write!(f, "Key({:?})", key_event),
      Self::Shutdown => write!(f, "Shutdown"),
      Self::ResetExpansionNetwork => write!(f, "ResetExpansionNetwork"),
      Self::StateTransition(_) => write!(f, "StateTransition(...)"),
    }
  }
}
