use std::any::{Any, TypeId};

use fast_protocol::SwitchState;

use crate::machine::event::Event;
use crate::prelude::*;

pub enum AppMessage {
  EmitEvent(Box<dyn Event>),
  // RegisterCommand(TypeId, Box<dyn Fn(&dyn Any, &mut Context) + Send + Sync>),
  RegisterCommand(
    TypeId,
    Box<dyn for<'ctx> Fn(&dyn Any, &mut Context<'ctx>) + Send + Sync>,
  ),
  ExecuteCommand(u64, Box<dyn Command>),
  SystemTick,
  Shutdown,
  SwitchStates(Vec<SwitchState>),
}
