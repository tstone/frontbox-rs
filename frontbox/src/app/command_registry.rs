use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::prelude::Context;

pub struct CommandRegistry {
  commands: HashMap<TypeId, u64>,
}

impl CommandRegistry {
  pub fn new() -> Self {
    Self {
      commands: HashMap::new(),
    }
  }

  pub fn register(&mut self, type_id: TypeId, system_id: u64) {
    self.commands.insert(type_id, system_id);
  }

  pub fn unregister(&mut self, type_id: TypeId) {
    self.commands.remove(&type_id);
  }

  pub fn unregister_by_system(&mut self, system_id: u64) {
    self.commands.retain(|_, entry| *entry != system_id);
  }

  pub fn get_system_for_command(&self, type_id: TypeId) -> Option<u64> {
    self.commands.get(&type_id).cloned()
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
