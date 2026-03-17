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
pub trait System {
  fn on_startup(&mut self, ctx: &mut Context) {}
  fn on_shutdown(&mut self, ctx: &mut Context) {}
  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {}
  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {}
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {}
  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {}

  fn on_interrupt(&mut self, event: &dyn Event, ctx: &mut Context) -> InterruptResult {
    InterruptResult::Continue
  }

  fn is_active(&self, ctx: &Context) -> bool {
    true
  }

  fn leds(&mut self, delta_time: Duration, ctx: &Context) -> HashMap<&'static str, LedState> {
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
  fn on_startup(&mut self, ctx: &mut Context) {
    self.as_system().on_startup(ctx);
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    self.as_system().on_shutdown(ctx);
  }

  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {
    self.as_system().on_timer(timer_name, ctx);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.as_system().on_tick(delta, ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    self.as_system().on_event(event, ctx);
  }

  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {
    self.as_system().on_command(command, ctx);
  }
}

/// A system which can be spawned dynamically during runtime and cloned (used as a template). Typically systems that implement game modes which are cloned, one per user, implement this.
pub trait ChildSystem: System + Send + Sync + DynClone {
  fn as_system(&mut self) -> &mut dyn System;
}

impl System for Box<dyn ChildSystem> {
  fn on_startup(&mut self, ctx: &mut Context) {
    self.as_system().on_startup(ctx);
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    self.as_system().on_shutdown(ctx);
  }

  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {
    self.as_system().on_timer(timer_name, ctx);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.as_system().on_tick(delta, ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    self.as_system().on_event(event, ctx);
  }

  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {
    self.as_system().on_command(command, ctx);
  }
}

dyn_clone::clone_trait_object!(ChildSystem);
