use std::collections::HashMap;
use std::time::Duration;

use dyn_clone::DynClone;

use crate::prelude::*;

/// A System responds to incoming events and enqueues commands
#[allow(unused)]
pub trait System: DynClone + Send + Sync {
  // TODO: convert to Frontbox Event
  fn on_config_change(&mut self, config_key: &'static str, ctx: &mut Context) {}

  fn on_startup(&mut self, ctx: &mut Context) {}
  fn on_shutdown(&mut self, ctx: &mut Context) {}

  fn leds(&mut self, delta_time: Duration) -> HashMap<&'static str, LedState> {
    HashMap::new()
  }
}

dyn_clone::clone_trait_object!(System);
