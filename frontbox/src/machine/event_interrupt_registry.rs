use std::any::TypeId;
use std::collections::HashMap;

pub struct Interrupt {
  pub system_id: u64,
  pub priority: u16,
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

  pub fn register(&mut self, type_id: TypeId, system_id: u64, priority: u16) {
    let interrupt = Interrupt {
      system_id,
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

  pub fn unregister_by_system(&mut self, system_id: u64) {
    for interrupts in self.interrupts.values_mut() {
      interrupts.retain(|i| i.system_id != system_id);
    }
  }

  pub fn get_interrupts_for_event(&self, event_type: TypeId) -> Option<&Vec<Interrupt>> {
    self.interrupts.get(&event_type)
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptResult {
  Continue,
  Halt,
}
