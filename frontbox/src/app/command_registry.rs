use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::prelude::Context;

struct CommandEntry {
  system_id: u64,
  runner: Box<dyn for<'ctx> Fn(&dyn Any, u64, &mut Context<'ctx>) + Send + Sync>,
}

pub struct CommandRegistry {
  commands: HashMap<TypeId, CommandEntry>,
}

impl CommandRegistry {
  pub fn new() -> Self {
    Self {
      commands: HashMap::new(),
    }
  }

  pub fn register(
    &mut self,
    type_id: TypeId,
    system_id: u64,
    runner: Box<dyn Fn(&dyn Any, u64, &mut Context) + Send + Sync>,
  ) {
    let entry = CommandEntry { system_id, runner };
    self.commands.insert(type_id, entry);
  }

  pub fn unregister(&mut self, type_id: TypeId) {
    self.commands.remove(&type_id);
  }

  pub fn unregister_by_system(&mut self, system_id: u64) {
    self
      .commands
      .retain(|_, entry| entry.system_id != system_id);
  }

  pub fn execute<C: Command + 'static>(&self, command: &C, caller_id: u64, context: &mut Context) {
    if let Some(entry) = self.commands.get(&TypeId::of::<C>()) {
      (entry.runner)(command, caller_id, context);
    } else {
      panic!(
        "No runner registered for command type {:?}",
        TypeId::of::<C>()
      );
    }
  }
}

pub trait Command: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Command for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[allow(type_alias_bounds)]
pub type CommandRunner<C: Command> = dyn Fn(&C, &mut Context) + Send + Sync;
