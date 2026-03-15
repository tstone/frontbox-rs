use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::prelude::Context;

pub struct CommandRegistry {
  commands: HashMap<TypeId, Box<dyn for<'ctx> Fn(&dyn Any, &mut Context<'ctx>) + Send + Sync>>,
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
    runner: Box<dyn Fn(&dyn Any, &mut Context) + Send + Sync>,
  ) {
    self.commands.insert(type_id, runner);
  }

  pub fn execute<C: Command + 'static>(&self, command: &C, context: &mut Context) {
    if let Some(runner) = self.commands.get(&TypeId::of::<C>()) {
      runner(command, context);
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
