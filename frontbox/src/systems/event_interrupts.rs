//! # Event Interrupts
//!
//! Sometimes there are cases where the normal flow of operation needs to be halted. For example, if a player drains while ball save is active, this would _normally_ emit an event that the player has drained and the turn is over. In these cases it's necessary to allow a system to override this behavior. This happens by way of event interrupts.
//!
//! Systems can register themselves as an event interrupt. Interrupt registration requires a priority. The framework will interrupts in priority order (highest first). This allows, for example, a temporary start-of-ball ball save to take precedence over an extra ball or outlane ball save.
//!
//! Event interrupts can be applied to any event within the system.
//!
//! ```rust
//! fn on_spawn(&mut self, ctx: &Context) {
//!   ctx.register_interrupt::<TroughFull>(100); // 100 is the priority
//! }
//!
//! fn on_interrupt(&mut self, event: &dyn Signal, ctx: &mut Context) -> InterruptResult {
//!   // interrupt handlers must return a result
//!   InterruptResult::Continue // or InterruptResult::Halt
//! }
//! ```

use std::any::TypeId;
use std::collections::HashMap;

use crate::prelude::SystemHandle;

#[derive(Debug)]
pub(crate) struct Interrupt {
  pub system_id: u64,
  pub parent_key: &'static str,
  pub priority: u16,
}

impl Interrupt {
  pub fn to_handle(&self) -> SystemHandle {
    SystemHandle::new(self.system_id, self.parent_key)
  }
}

#[derive(Debug)]
pub(crate) struct EventInterruptRegistry {
  interrupts: HashMap<TypeId, Vec<Interrupt>>,
}

impl EventInterruptRegistry {
  pub fn new() -> Self {
    Self {
      interrupts: HashMap::new(),
    }
  }

  pub fn register(
    &mut self,
    type_id: TypeId,
    system_id: u64,
    parent_key: &'static str,
    priority: u16,
  ) {
    let interrupt = Interrupt {
      system_id,
      parent_key,
      priority,
    };

    let interrupts = self.interrupts.entry(type_id).or_default();
    let pos = interrupts.partition_point(|i| i.priority >= priority);
    interrupts.insert(pos, interrupt);
  }

  pub fn unregister(&mut self, system_id: u64, event_type: TypeId) {
    if let Some(interrupts) = self.interrupts.get_mut(&event_type) {
      interrupts.retain(|i| i.system_id != system_id);
    }
  }

  pub fn unregister_by_system(&mut self, system_id: &u64) {
    for interrupts in self.interrupts.values_mut() {
      interrupts.retain(|i| &i.system_id != system_id);
    }
  }

  pub fn get_interrupts_for_event(&self, event_type: TypeId) -> Option<&Vec<Interrupt>> {
    self.interrupts.get(&event_type)
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize)]
pub enum InterruptResult {
  /// Event is broadcast to all systems (default operation)
  Continue,
  /// Event is not broadcast. Only the interrupting system has seen the event.
  Halt,
}
