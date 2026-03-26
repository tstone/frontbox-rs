use std::ops::{Deref, DerefMut};

use crate::prelude::*;

pub struct SystemGroup {
  pub(crate) systems: Systems,
  active: bool,
}

impl SystemGroup {
  pub fn new(containers: Vec<SystemContainer>) -> Self {
    let mut systems = Systems::new();
    for system in containers {
      systems.insert(system);
    }

    Self {
      systems,
      active: true,
    }
  }

  pub fn activate(&mut self, ctx: &Context, systems: &Systems) {
    if !self.active {
      log::info!("Activating system group");
      for mut system in self.systems.values_mut() {
        let mut ctx = ctx.clone_for_system(system.id());
        // only emit reactivate to systems that are also individually active
        if system.is_active(&ctx, systems) {
          system.on_reactivate(&mut ctx, systems);
        }
      }
      self.active = true;
    }
  }

  pub fn deactivate(&mut self, ctx: &Context, systems: &Systems) {
    if self.active {
      log::info!("Deactivating system group");
      for mut system in self.systems.values_mut() {
        let mut ctx = ctx.clone_for_system(system.id());
        // only emit deactivate to systems that are also individually active (since they otherwise would have been active)
        if system.is_active(&ctx, systems) {
          log::trace!("System {} is active, deactivating", system.id());
          system.on_deactivate(&mut ctx, systems);
        }
      }
      self.active = false;
    }
  }
}

impl System for SystemGroup {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    for mut system in self.systems.values_mut() {
      let mut ctx = ctx.clone_for_system(system.id());
      system.on_startup(&mut ctx, systems);
    }
  }

  fn on_shutdown(&mut self, ctx: &Context, systems: &Systems) {
    for mut system in self.systems.values_mut() {
      let mut ctx = ctx.clone_for_system(system.id());
      system.on_shutdown(&mut ctx, systems);
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
    for mut system in self.systems.values_mut() {
      let mut ctx = ctx.clone_for_system(system.id());
      if system.handle_active(&mut ctx, systems) {
        system.on_tick(delta, &mut ctx, systems);
      } else {
        log::trace!("System {} is inactive, skipping tick", system.id(),);
      }
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    for mut system in self.systems.values_mut() {
      let mut ctx = ctx.clone_for_system(system.id());
      if system.handle_active(&mut ctx, systems) {
        system.on_event(event, &mut ctx, systems);
      } else {
        log::trace!(
          "System {} is inactive, skipping event of type {:?}",
          system.id(),
          event.type_id()
        );
        if system.handle_active(&mut ctx, systems) {
          log::trace!(
            "System {} was active but is now inactive, deactivating",
            system.id()
          );
          system.on_deactivate(&mut ctx, systems);
        }
      }
    }
  }

  fn on_deactivate(&mut self, ctx: &Context, systems: &Systems) {
    self.deactivate(ctx, systems);
  }

  fn on_reactivate(&mut self, ctx: &Context, systems: &Systems) {
    self.activate(ctx, systems);
  }

  fn is_active(&self, _ctx: &Context, _systems: &Systems) -> bool {
    self.active
  }
}

impl Deref for SystemGroup {
  type Target = Systems;

  fn deref(&self) -> &Self::Target {
    &self.systems
  }
}

impl DerefMut for SystemGroup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.systems
  }
}
