use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::{ChildSystem, System, SystemContainer};

pub struct SystemGroup {
  pub(crate) systems: HashMap<u64, SystemContainer>,
  active: bool,
}

impl SystemGroup {
  pub fn new(systems: Vec<Box<dyn ChildSystem>>) -> Self {
    let mut system_map = HashMap::new();
    for system in systems {
      let system: Box<dyn System> = Box::new(system);
      let container = SystemContainer::new_from_system(system);
      system_map.insert(container.id, container);
    }

    Self {
      systems: system_map,
      active: true,
    }
  }

  pub fn activate(&mut self) {
    self.active = true;
  }

  pub fn deactivate(&mut self) {
    self.active = false;
  }
}

impl System for SystemGroup {
  fn on_startup(&mut self, ctx: &mut super::Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      system.on_startup(&mut ctx);
    }
  }

  fn on_shutdown(&mut self, ctx: &mut super::Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      system.on_shutdown(&mut ctx);
    }
  }

  fn on_tick(&mut self, delta: std::time::Duration, ctx: &mut super::Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if system.is_active(&ctx) {
        system.on_tick(delta, &mut ctx);
      }
    }
  }

  fn on_event(&mut self, event: &dyn crate::prelude::Event, ctx: &mut super::Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if system.is_active(&ctx) {
        system.on_event(event, &mut ctx);
      }
    }
  }

  fn is_active(&self, _ctx: &super::Context) -> bool {
    self.active
  }
}

impl Deref for SystemGroup {
  type Target = HashMap<u64, SystemContainer>;

  fn deref(&self) -> &Self::Target {
    &self.systems
  }
}

impl DerefMut for SystemGroup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.systems
  }
}
