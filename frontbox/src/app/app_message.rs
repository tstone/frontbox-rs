use std::any::{TypeId, type_name};
use std::fmt::Display;

use fast_protocol::SwitchState;

use crate::prelude::*;

pub enum AppMessage {
  EmitEvent(EventBox),
  RegisterInterrupt(SystemHandle, TypeId, u16),
  UnregisterInterrupt(u64, TypeId),
  /// Unregister all everything associated with the given system ID. This is useful for cleaning up when a system is removed.
  UnregisterAllBySystem(u64),
  SystemTick,
  Shutdown,
  SingleSwitchState(usize, SwitchState),
  SwitchStates(Vec<SwitchState>),
  SpawnSystem(&'static str, SpawnableSystemContainer),
  ReplaceSystem(SystemHandle, SpawnableSystemContainer),
  DespawnSystem(SystemHandle),
  SpawnSystemGroup(&'static str, Vec<ChildSystemContainer>, bool),
  DespawnSystemGroup(&'static str),
  ActivateSystemGroup(&'static str),
  DeactivateSystemGroup(&'static str),
  CreateCue(SystemHandle, u64, Cue, Vec<Box<dyn Event>>),
  CreateCueTimeline(SystemHandle, u64, CueTimeline),
  CancelCue(SystemHandle, u64),
}

impl Display for AppMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AppMessage::EmitEvent(event) => write!(f, "EmitEvent({})", event.type_name),
      AppMessage::RegisterInterrupt(handle, type_id, priority) => {
        write!(
          f,
          "RegisterInterrupt({}, {:?}, {})",
          handle.id, type_id, priority
        )
      }
      AppMessage::UnregisterInterrupt(id, type_id) => {
        write!(f, "UnregisterInterrupt({}, {:?})", id, type_id)
      }
      AppMessage::UnregisterAllBySystem(id) => write!(f, "UnregisterAllBySystem({})", id),
      AppMessage::SystemTick => write!(f, "SystemTick"),
      AppMessage::Shutdown => write!(f, "Shutdown"),
      AppMessage::SingleSwitchState(index, state) => {
        write!(f, "SingleSwitchState({}, {:?})", index, state)
      }
      AppMessage::SwitchStates(states) => write!(f, "SwitchStates({:?})", states),
      AppMessage::SpawnSystem(parent_key, _) => write!(f, "SpawnSystem({:?})", parent_key),
      AppMessage::ReplaceSystem(handle, _) => {
        write!(f, "ReplaceSystem({}, {})", handle.id, handle.parent_key)
      }
      AppMessage::DespawnSystem(handle) => {
        write!(f, "DespawnSystem({}, {})", handle.id, handle.parent_key)
      }
      AppMessage::SpawnSystemGroup(name, _, exclusive) => {
        write!(f, "SpawnSystemGroup({}, exclusive={})", name, exclusive)
      }
      AppMessage::DespawnSystemGroup(name) => write!(f, "DespawnSystemGroup({})", name),
      AppMessage::ActivateSystemGroup(name) => write!(f, "ActivateSystemGroup({})", name),
      AppMessage::DeactivateSystemGroup(name) => write!(f, "DeactivateSystemGroup({})", name),
      AppMessage::CreateCue(handle, cue_id, cue, _signals) => {
        write!(f, "CreateCue({}:{}, {:?})", handle.id, cue_id, cue)
      }
      AppMessage::CreateCueTimeline(handle, cue_id, _timeline) => {
        write!(f, "CreateCueTimeline({}:{})", handle.id, cue_id)
      }
      AppMessage::CancelCue(handle, cue_id) => write!(f, "CancelCue({}:{})", handle.id, cue_id),
    }
  }
}

pub struct EventBox {
  pub event: Box<dyn Event>,
  pub type_id: TypeId,
  pub type_name: &'static str,
}

impl EventBox {
  pub fn new<E: Event>(event: E) -> Self {
    EventBox {
      type_id: event.type_id(),
      type_name: type_name::<E>(),
      event: Box::new(event),
    }
  }
}
