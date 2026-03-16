use std::any::TypeId;
use std::collections::HashMap;

use crate::prelude::*;

struct Interrupt {
  system_id: u64,
  priority: u16,
  handler: Box<dyn for<'ctx> Fn(&dyn Event, &mut Context<'ctx>) -> InterruptResult + Send + Sync>,
}

pub struct EventInterruptRegistry {
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
    priority: u16,
    interrupt: Box<dyn Fn(&dyn Event, &mut Context) -> InterruptResult + Send + Sync>,
  ) {
    let interrupt = Interrupt {
      system_id,
      priority,
      handler: Box::new(move |event, context| interrupt(event, context)),
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

  pub fn unregister_by_system(&mut self, system_id: u64) {
    for interrupts in self.interrupts.values_mut() {
      interrupts.retain(|i| i.system_id != system_id);
    }
  }

  /// Invoke all registered interrupts. If any interrupt returns `Halt`, stop processing further.
  pub fn handle(&self, event: &dyn Event, ctx_template: &mut Context) -> InterruptResult {
    if let Some(interrupt) = self.interrupts.get(&event.as_any().type_id()) {
      for interrupt in interrupt.iter() {
        let mut ctx = ctx_template.clone_for_system(interrupt.system_id);
        let result = (interrupt.handler)(event, &mut ctx);
        if result == InterruptResult::Halt {
          return InterruptResult::Halt;
        }
      }
    }
    InterruptResult::Continue
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptResult {
  Continue,
  Halt,
}
