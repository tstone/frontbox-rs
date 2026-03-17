use std::any::TypeId;

use fast_protocol::SwitchState;

use crate::machine::event::Event;
use crate::prelude::*;

pub enum AppMessage {
  EmitEvent(Box<dyn Event>),
  ExecuteCommand(u64, Box<dyn Command>),
  RegisterCommand(u64, TypeId),
  RegisterInterrupt(u64, TypeId, u16),
  UnregisterInterrupt(u64, TypeId),
  UnregisterCommand(u64, TypeId),
  /// Unregister all everything associated with the given system ID. This is useful for cleaning up when a system is removed.
  UnregisterAllBySystem(u64),
  SystemTick,
  Shutdown,
  SwitchStates(Vec<SwitchState>),
  SpawnSystem(u64, Box<dyn SpawnableSystem>),
  ReplaceSystem(u64, Box<dyn SpawnableSystem>),
  DespawnSystem(u64),
  SpawnSystemGroup(&'static str, Vec<Box<dyn ChildSystem>>, bool),
  DespawnSystemGroup(&'static str),
  ActivateSystemGroup(&'static str),
  DeactivateSystemGroup(&'static str),
  ClearTimer(u64, &'static str),
  SetTimer(u64, &'static str, Duration, TimerMode),
}
