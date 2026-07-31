use std::any::{TypeId, type_name};
use std::fmt::Display;

use fast_protocol::SwitchState;

use crate::prelude::*;

pub enum AppMessage {
  EmitEvent(EventBox),
  RegisterInterrupt(u64, TypeId, u16),
  UnregisterInterrupt(u64, TypeId),
  /// Unregister all everything associated with the given system ID. This is useful for cleaning up when a system is removed.
  UnregisterAllBySystem(u64),
  SystemTick,
  Shutdown,
  SingleSwitchState(usize, SwitchState),
  SwitchStates(Vec<SwitchState>),
  SpawnSystem(u64, SpawnableSystemContainer),
  ReplaceSystem(u64, SpawnableSystemContainer),
  DespawnSystem(u64),
  SpawnSystemGroup(&'static str, Vec<ChildSystemContainer>, bool),
  DespawnSystemGroup(&'static str),
  ActivateSystemGroup(&'static str),
  DeactivateSystemGroup(&'static str),
  CreateCue(u64, u64, Cue, Vec<Box<dyn Event>>),
  CreateCueTimeline(u64, u64, CueTimeline),
  CancelCue(u64, u64),
}

impl Display for AppMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AppMessage::EmitEvent(event) => write!(f, "EmitEvent({})", event.type_name),
      AppMessage::RegisterInterrupt(id, type_id, priority) => {
        write!(f, "RegisterInterrupt({}, {:?}, {})", id, type_id, priority)
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
      AppMessage::SpawnSystem(id, _) => write!(f, "SpawnSystem({})", id),
      AppMessage::ReplaceSystem(id, _) => write!(f, "ReplaceSystem({})", id),
      AppMessage::DespawnSystem(id) => write!(f, "DespawnSystem({})", id),
      AppMessage::SpawnSystemGroup(name, _, exclusive) => {
        write!(f, "SpawnSystemGroup({}, exclusive={})", name, exclusive)
      }
      AppMessage::DespawnSystemGroup(name) => write!(f, "DespawnSystemGroup({})", name),
      AppMessage::ActivateSystemGroup(name) => write!(f, "ActivateSystemGroup({})", name),
      AppMessage::DeactivateSystemGroup(name) => write!(f, "DeactivateSystemGroup({})", name),
      AppMessage::CreateCue(system_id, cue_id, cue, _signals) => {
        write!(f, "CreateCue({}:{}, {:?})", system_id, cue_id, cue)
      }
      AppMessage::CreateCueTimeline(system_id, cue_id, _timeline) => {
        write!(f, "CreateCueTimeline({}:{})", system_id, cue_id)
      }
      AppMessage::CancelCue(system_id, cue_id) => write!(f, "CancelCue({}:{})", system_id, cue_id),
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
      event: Box::new(event)
    }
  }
}