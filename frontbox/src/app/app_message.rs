use std::any::{Any, TypeId};

use fast_protocol::SwitchState;

use crate::machine::event::Event;
use crate::machine::event_interrupt_registry::InterruptResult;
use crate::prelude::*;

pub enum AppMessage {
  EmitEvent(Box<dyn Event>),
  ExecuteCommand(u64, Box<dyn Command>),
  RegisterCommand(
    u64,
    TypeId,
    Box<dyn for<'ctx> Fn(&dyn Any, u64, &mut Context<'ctx>) + Send + Sync>,
  ),
  RegisterInterrupt(
    u64,
    TypeId,
    u16,
    Box<dyn for<'ctx> Fn(&dyn Event, &mut Context) -> InterruptResult + Send + Sync>,
  ),
  UnregisterInterrupt(u64, TypeId),
  UnregisterCommand(u64, TypeId),
  /// Unregister all everything associated with the given system ID. This is useful for cleaning up when a system is removed.
  UnregisterAllBySystem(u64),
  // TODO: add commands
  SystemTick,
  Shutdown,
  SwitchStates(Vec<SwitchState>),
}
