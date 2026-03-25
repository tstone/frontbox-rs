use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;

use dyn_clone::DynClone;

use crate::prelude::*;

/// A `System` is a basic building block of Frontbox which responds to events, schedules timers, registers command, and handles interrupts.
///
/// The base system does not need to be thread safe (Send+Sync), though this can only be spun up at boot time.
/// Spawning systems dynamically during runtime requires the system to be Send+Sync. Some systems can be managed as others, or used as
/// templates. These additionally require `Clone`.
#[allow(unused)]
pub trait System: Any {
  /// Called when the system is first started up
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {}
  /// Called when the system is deactivated. This also includes if a parent group is deactivated (activation bubbles)
  fn on_deactivate(&mut self, ctx: &Context, systems: &Systems) {}
  /// Called when the system is re-activated after being deactivated. This also includes if a parent group is re-activated (activation bubbles)
  fn on_reactivate(&mut self, ctx: &Context, systems: &Systems) {}
  /// Called when the system is removed
  fn on_shutdown(&mut self, ctx: &Context, systems: &Systems) {}

  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {}
  fn on_event(&mut self, event: &dyn Signal, ctx: &Context, systems: &Systems) {}
  fn on_cue(&mut self, cue: &dyn Signal, ctx: &Context, systems: &Systems) {}
  fn on_interrupt(&mut self, event: &dyn Signal, ctx: &Context) -> InterruptResult {
    InterruptResult::Continue
  }

  fn is_active(&self, ctx: &Context, systems: &Systems) -> bool {
    true
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    ctx: &Context,
    systems: &Systems,
  ) -> HashMap<&'static str, LedState> {
    HashMap::new()
  }
}

/// A system which can be spawned dynamically during runtime.
pub trait SpawnableSystem: System + Send + Sync {
  fn as_system(&mut self) -> &mut dyn System;
}

impl<T: System + Send + Sync> SpawnableSystem for T {
  fn as_system(&mut self) -> &mut dyn System {
    self
  }
}

impl System for Box<dyn SpawnableSystem> {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    self.as_system().on_startup(ctx, systems);
  }

  fn on_shutdown(&mut self, ctx: &Context, systems: &Systems) {
    self.as_system().on_shutdown(ctx, systems);
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &Context, systems: &Systems) {
    self.as_system().on_cue(cue, ctx, systems);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
    self.as_system().on_tick(delta, ctx, systems);
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &Context, systems: &Systems) {
    self.as_system().on_event(event, ctx, systems);
  }
}

/// A system which can be spawned dynamically during runtime and cloned (used as a template). Typically systems that implement game modes which are cloned, one per user, implement this.
pub trait ChildSystem: System + Send + Sync + DynClone {
  fn as_system(&mut self) -> &mut dyn System;
}

impl System for Box<dyn ChildSystem> {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    self.as_system().on_startup(ctx, systems);
  }

  fn on_shutdown(&mut self, ctx: &Context, systems: &Systems) {
    self.as_system().on_shutdown(ctx, systems);
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &Context, systems: &Systems) {
    self.as_system().on_cue(cue, ctx, systems);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
    self.as_system().on_tick(delta, ctx, systems);
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &Context, systems: &Systems) {
    self.as_system().on_event(event, ctx, systems);
  }
}

dyn_clone::clone_trait_object!(ChildSystem);
