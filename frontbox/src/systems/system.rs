use std::any::Any;
use std::time::Duration;

use dyn_clone::DynClone;

use crate::operator_config::GeneralizedConfigValue;
use crate::prelude::*;

/// A `System` is a basic building block of Frontbox which responds to events, schedules timers, registers command, and handles interrupts.
///
/// The base system does not need to be thread safe (Send+Sync), though this can only be spun up at boot time.
/// Spawning systems dynamically during runtime requires the system to be Send+Sync. Some systems can be managed as others, or used as
/// templates. These additionally require `Clone`.
#[allow(unused)]
pub trait System: Any {
  /// Called when the system is first started up
  fn on_spawn(&mut self, ctx: &SystemContext) {}
  /// Called when the system is removed
  fn on_despawn(&mut self, ctx: &SystemContext) {}

  /// Called when the system is deactivated. This also includes if a parent group is deactivated (activation bubbles)
  fn on_deactivate(&mut self, ctx: &SystemContext) {
    if let Some(mut led_system) = ctx.get::<LedSystem>() {
      led_system.deactivate_by_system(ctx.current_system_id());
    }
  }

  /// Called when the system is re-activated after being deactivated. This also includes if a parent group is re-activated (activation bubbles)
  fn on_reactivate(&mut self, ctx: &SystemContext) {
    if let Some(mut led_system) = ctx.get::<LedSystem>() {
      led_system.activate_by_system(ctx.current_system_id());
    }
  }

  /// On tick is called every system tick, which is typically 30-60Hz, but can be configured by the app. This is where most of the game logic should go.
  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {}
  /// On render is called every system tick after on_tick, and is where rendering (LEDs, DMD, screen, etc.) should be handled since on_tick processed all state needed for rendering.
  fn on_render(&mut self, ctx: &SystemContext) {}

  /// Called when an event is emitted into the system (switch press, game state change, etc.)
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {}
  /// Invoked by the framework to handle an potential interrupt
  fn on_interrupt(&mut self, event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
    InterruptResult::Continue
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    true
  }

  fn config_values(&self) -> Vec<&'static dyn GeneralizedConfigValue> {
    Vec::new()
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
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.as_system().on_spawn(ctx);
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    self.as_system().on_despawn(ctx);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    self.as_system().on_tick(delta, ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    self.as_system().on_event(event, ctx);
  }
}

/// A system which can be spawned dynamically during runtime and cloned (used as a template). Typically systems that implement game modes which are cloned, one per user, implement this.
pub trait ChildSystem: System + Send + Sync + DynClone {
  fn as_system(&mut self) -> &mut dyn System;
}

impl System for Box<dyn ChildSystem> {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    <dyn ChildSystem>::as_system(self).on_spawn(ctx);
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    <dyn ChildSystem>::as_system(self).on_despawn(ctx);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    <dyn ChildSystem>::as_system(self).on_tick(delta, ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    <dyn ChildSystem>::as_system(self).on_event(event, ctx);
  }
}

impl<T: System + Send + Sync> ChildSystem for T
where
  T: Clone,
{
  fn as_system(&mut self) -> &mut dyn System {
    self
  }
}

dyn_clone::clone_trait_object!(ChildSystem);
