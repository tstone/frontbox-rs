use std::collections::HashMap;
use std::time::Duration;

use dyn_clone::DynClone;

use crate::prelude::*;

/// A System responds to incoming events and enqueues commands
#[allow(unused)]
pub trait System {
  fn on_startup(&mut self, ctx: &mut Context) {}
  fn on_shutdown(&mut self, ctx: &mut Context) {}
  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {}
  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {}
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {}
  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {}
  fn on_interrupt(&mut self, event: &dyn Event, ctx: &mut Context) {}

  fn is_active(&self, ctx: &Context) -> bool {
    true
  }

  fn leds(&mut self, delta_time: Duration, ctx: &Context) -> HashMap<&'static str, LedState> {
    HashMap::new()
  }
}

/// A ChildSystem defines the behavior of a system that can be cloned and managed
#[allow(unused)]
pub trait ChildSystem: DynClone + Send + Sync {
  fn on_startup(&mut self, ctx: &mut Context) {}
  fn on_shutdown(&mut self, ctx: &mut Context) {}
  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {}
  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {}
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {}
  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {}
  fn on_interrupt(&mut self, event: &dyn Event, ctx: &mut Context) {}

  fn is_active(&self, ctx: &Context) -> bool {
    true
  }

  fn leds(&mut self, delta_time: Duration, ctx: &Context) -> HashMap<&'static str, LedState> {
    HashMap::new()
  }
}

dyn_clone::clone_trait_object!(ChildSystem);

impl<T: ChildSystem> System for T {}

impl System for Box<dyn ChildSystem> {
  fn on_startup(&mut self, ctx: &mut Context) {
    self.as_mut().on_startup(ctx);
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    self.as_mut().on_shutdown(ctx);
  }

  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {
    self.as_mut().on_timer(timer_name, ctx);
  }

  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.as_mut().on_tick(delta, ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    self.as_mut().on_event(event, ctx);
  }

  fn is_active(&self, ctx: &Context) -> bool {
    self.as_ref().is_active(ctx)
  }

  fn leds(&mut self, delta_time: Duration, ctx: &Context) -> HashMap<&'static str, LedState> {
    self.as_mut().leds(delta_time, ctx)
  }
}
