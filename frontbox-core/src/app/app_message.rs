use std::any::TypeId;
use std::fmt::Debug;

use fast_protocol::SwitchState;

use crate::prelude::app_tracer::TraceEvent;
use crate::prelude::*;
use crate::systems::routine::RoutineStart;
use crate::systems::state::StoredState;

pub enum AppMessage {
  EmitEvent(Option<u64>, EventBox),
  // Broadcast an event emitted by the given system to it first, then queue `EmitEventToOthers` for everyone else, so
  // whatever it queues in response lands before they get it (run loop internal)
  EmitEventOriginFirst(SystemHandle, EventBox),
  // The rest of an `EmitEventOriginFirst`: broadcast to every system but the given one, which already handled it (run loop internal)
  EmitEventToOthers(SystemHandle, EventBox),
  Shutdown(ShutdownScope),
  SwitchStateChange(usize, SwitchState),
  SyncSwitchStates(Vec<SwitchState>),
  MachineDisconnected,
  /// The machine reconnected to the hardware, which reported these switch states
  MachineReconnected(Vec<SwitchState>),
  ConfigChanged(&'static str),
  DriverModeChange(usize, DriverMode),
  SpawnSystem(GroupId, SpawnableSystemContainer),
  ReplaceSystem(SystemHandle, SpawnableSystemContainer),
  DespawnSystem(SystemHandle),
  /// Spawn a group owned by the system
  SpawnSystemGroup(SystemHandle, GroupHandle, Vec<ChildSystemContainer>, bool),
  DespawnSystemGroup(GroupId),
  ActivateSystemGroup(GroupId),
  DeactivateSystemGroup(GroupId),
  CreateCue(SystemHandle, u64, Cue, Vec<Box<dyn Event>>),
  CancelCue(SystemHandle, u64),
  StartRoutine(RoutineStart),
  /// The origin cancels the routine with this id
  CancelRoutine(SystemHandle, u64),
  CompleteRoutineStep(SystemHandle, TypeId),
  HaltRoutine(SystemHandle, TypeId),
  /// Hand a started routine to its first participant, once its `RoutineStarted` has gone out
  BeginRoutine(TypeId),
  /// A system's own `is_active` changed, noticed while handling something else
  SystemActivityChanged(SystemHandle),
  SetState(StoredState),
  TracerEvent(TraceEvent),
}

impl Debug for AppMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AppMessage::EmitEvent(sender, event) => {
        write!(f, "EmitEvent({:?}, {})", sender, event.type_name)
      }
      AppMessage::EmitEventOriginFirst(origin, event) => {
        write!(
          f,
          "EmitEventOriginFirst({}, {})",
          origin.id, event.type_name
        )
      }
      AppMessage::EmitEventToOthers(handled_by, event) => {
        write!(
          f,
          "EmitEventToOthers({}, {})",
          handled_by.id, event.type_name
        )
      }
      AppMessage::Shutdown(_) => write!(f, "Shutdown"),
      AppMessage::SwitchStateChange(index, state) => {
        write!(f, "SingleSwitchState({}, {:?})", index, state)
      }
      AppMessage::SyncSwitchStates(states) => write!(f, "SwitchStates({:?})", states),
      AppMessage::MachineDisconnected => write!(f, "MachineDisconnected"),
      AppMessage::MachineReconnected(states) => write!(f, "MachineReconnected({:?})", states),
      AppMessage::ConfigChanged(key) => write!(f, "ConfigChanged({})", key),
      AppMessage::DriverModeChange(driver, mode) => {
        write!(f, "DriverModeChange({}, {:?})", driver, mode)
      }
      AppMessage::SpawnSystem(group, _) => write!(f, "SpawnSystem({})", group),
      AppMessage::ReplaceSystem(handle, _) => {
        write!(f, "ReplaceSystem({}, {})", handle.id, handle.group)
      }
      AppMessage::DespawnSystem(handle) => {
        write!(f, "DespawnSystem({}, {})", handle.id, handle.group)
      }
      AppMessage::SpawnSystemGroup(owner, group, _, exclusive) => {
        write!(
          f,
          "SpawnSystemGroup({} {}, owner={}:{}, exclusive={})",
          group.id(),
          group.name(),
          owner.group,
          owner.id,
          exclusive
        )
      }
      AppMessage::DespawnSystemGroup(id) => write!(f, "DespawnSystemGroup({})", id),
      AppMessage::ActivateSystemGroup(id) => write!(f, "ActivateSystemGroup({})", id),
      AppMessage::DeactivateSystemGroup(id) => write!(f, "DeactivateSystemGroup({})", id),
      AppMessage::CreateCue(handle, cue_id, cue, _signals) => {
        write!(f, "CreateCue({}:{}, {:?})", handle.id, cue_id, cue)
      }
      AppMessage::CancelCue(handle, cue_id) => write!(f, "CancelCue({}:{})", handle.id, cue_id),
      AppMessage::StartRoutine(start) => write!(f, "StartRoutine({:?})", start),
      AppMessage::CancelRoutine(handle, routine_id) => {
        write!(f, "CancelRoutine({}:{})", handle.id, routine_id)
      }
      AppMessage::CompleteRoutineStep(handle, type_id) => {
        write!(f, "CompleteRoutineStep({}, {:?})", handle.id, type_id)
      }
      AppMessage::HaltRoutine(handle, type_id) => {
        write!(f, "HaltRoutine({}, {:?})", handle.id, type_id)
      }
      AppMessage::BeginRoutine(type_id) => write!(f, "BeginRoutine({:?})", type_id),
      AppMessage::SystemActivityChanged(handle) => {
        write!(f, "SystemActivityChanged({}, {})", handle.id, handle.group)
      }
      AppMessage::SetState(state) => write!(f, "SetState({:?})", state),
      // An LED batch holds every changed LED and is sent several times a tick, so for performance simplify it
      AppMessage::TracerEvent(TraceEvent::LedsRGBChange {
        expansion,
        breakout,
        states,
      }) => write!(
        f,
        "TracerEvent(LedsRGBChange {{ expansion: {}, breakout: {:?}, {} leds }})",
        expansion,
        breakout,
        states.len()
      ),
      AppMessage::TracerEvent(event) => write!(f, "TracerEvent({:?})", event),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShutdownScope {
  /// Shutdown just the framework process
  Process,
  /// Stop the framework and the operating system
  OperatingSystem,
}
