use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

pub struct SystemGroup {
  pub(crate) systems: HashMap<u64, SystemContainer>,
  systems_active: HashMap<u64, bool>,
  active: bool,
}

impl SystemGroup {
  pub fn new(systems: Vec<Box<dyn ChildSystem>>, ctx_template: &mut Context) -> Self {
    let mut system_map = HashMap::new();
    let mut systems_active = HashMap::new();
    for system in systems {
      let container = SystemContainer::new_from_system(Box::new(system));
      let ctx = ctx_template.clone_for_system(container.id);
      systems_active.insert(container.id, container.is_active(&ctx));
      system_map.insert(container.id, container);
    }

    Self {
      systems: system_map,
      systems_active,
      active: true,
    }
  }

  pub fn activate(&mut self, ctx: &mut Context) {
    if !self.active {
      log::info!("Activating system group");
      for (id, system) in &mut self.systems {
        let mut ctx = ctx.clone_for_system(*id);
        // only emit reactivate to systems that are also individually active
        if system.is_active(&ctx) {
          log::trace!("System {} is active, activating", id);
          system.on_reactivate(&mut ctx);
          self.systems_active.insert(*id, true);
        }
      }
    }

    self.active = true;
  }

  pub fn deactivate(&mut self, ctx: &mut Context) {
    if self.active {
      log::info!("Deactivating system group");
      for (id, system) in &mut self.systems {
        let mut ctx = ctx.clone_for_system(*id);
        // only emit deactivate to systems that are also individually active (since they otherwise would have been active)
        if system.is_active(&ctx) {
          log::trace!("System {} is active, deactivating", id);
          system.on_deactivate(&mut ctx);
        }
      }
    }
    self.active = false;
  }

  /// Cycle through all systems and check if their active state has changed, firing the deactivation/reactivation handlers as needed.
  fn check_active_state(&mut self, ctx: &mut Context) {
    let mut reactivate_systems = Vec::new();
    let mut deactivate_systems = Vec::new();

    for (id, system) in &mut self.systems {
      let ctx = ctx.clone_for_system(*id);
      let currently_active = self.systems_active.get(id).copied().unwrap_or(false);
      let should_be_active = system.is_active(&ctx);

      if should_be_active && !currently_active {
        reactivate_systems.push(*id);
      } else if !should_be_active && currently_active {
        deactivate_systems.push(*id);
      }
    }

    for id in reactivate_systems {
      if let Some(system) = self.systems.get_mut(&id) {
        log::trace!("System {} is now active, activating", id);
        system.on_reactivate(ctx);
        self.systems_active.insert(id, true);
      }
    }

    for id in deactivate_systems {
      if let Some(system) = self.systems.get_mut(&id) {
        log::trace!("System {} is now inactive, deactivating", id);
        system.on_deactivate(ctx);
        self.systems_active.insert(id, false);
      }
    }
  }
}

impl System for SystemGroup {
  fn on_startup(&mut self, ctx: &mut Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      system.on_startup(&mut ctx);
    }
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      system.on_shutdown(&mut ctx);
    }
  }

  fn on_tick(&mut self, delta: std::time::Duration, ctx: &mut Context) {
    self.check_active_state(ctx);
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if self.systems_active.get(id) == Some(&true) {
        system.on_tick(delta, &mut ctx);
      } else {
        log::trace!("System {} is inactive, skipping tick", id,);
      }
    }
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
    self.check_active_state(ctx);
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if self.systems_active.get(id) == Some(&true) {
        system.on_event(event, &mut ctx);
      } else {
        log::trace!(
          "System {} is inactive, skipping event of type {:?}",
          id,
          event.type_id()
        );
        if self.systems_active.get(id) == Some(&true) {
          log::trace!("System {} was active but is now inactive, deactivating", id);
          system.on_deactivate(&mut ctx);
          self.systems_active.insert(*id, false);
        }
      }
    }
  }

  fn on_command(&mut self, command: &dyn Signal, ctx: &mut Context) {
    self.check_active_state(ctx);
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if self.systems_active.get(id) == Some(&true) {
        system.on_command(command, &mut ctx);
      } else {
        log::trace!(
          "System {} is inactive, skipping command of type {:?}",
          id,
          command.type_id()
        );
        if self.systems_active.get(id) == Some(&true) {
          log::trace!("System {} was active but is now inactive, deactivating", id);
          system.on_deactivate(&mut ctx);
          self.systems_active.insert(*id, false);
        }
      }
    }
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &mut Context) {
    self.check_active_state(ctx);
    for (id, system) in &mut self.systems {
      let mut ctx = ctx.clone_for_system(*id);
      if self.systems_active.get(id) == Some(&true) {
        system.on_cue(cue, &mut ctx);
      } else {
        log::trace!(
          "System {} is inactive, skipping cue of type {:?}",
          id,
          cue.type_id()
        );
        if self.systems_active.get(id) == Some(&true) {
          log::trace!("System {} was active but is now inactive, deactivating", id);
          system.on_deactivate(&mut ctx);
          self.systems_active.insert(*id, false);
        }
      }
    }
  }

  fn on_deactivate(&mut self, ctx: &mut Context) {
    self.deactivate(ctx);
  }

  fn on_reactivate(&mut self, ctx: &mut Context) {
    self.activate(ctx);
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
