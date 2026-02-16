use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use dyn_clone::DynClone;

use crate::led::LedDeclaration;
use crate::machine::system_timer::{SystemTimer, TimerMode};
use crate::prelude::*;

/// A System responds to incoming events and enqueues commands
#[allow(unused)]
pub trait System: DynClone + Send + Sync {
  /// Runs when a switch becomes closed (depressed)
  fn on_switch_closed(&mut self, switch: &Switch, ctx: &mut Context) {}
  /// Runs when a switch becomes open (released)
  fn on_switch_opened(&mut self, switch: &Switch, ctx: &mut Context) {}
  /// Runs when a timer completes
  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {}
  /// Runs whenever the system enters the scene, including at the start of a game
  fn on_system_enter(&mut self, ctx: &mut Context) {}
  /// Runs whenever the system exits the scene, including at the end of a game
  fn on_system_exit(&mut self, ctx: &mut Context) {}

  fn on_ball_start(&mut self, ctx: &mut Context) {}
  fn on_ball_end(&mut self, ctx: &mut Context) {}

  fn on_config_change(&mut self, config_key: &'static str, ctx: &mut Context) {}

  fn leds(&self) -> Vec<LedDeclaration> {
    vec![]
  }
}

pub struct SystemContainer {
  pub(crate) inner: Box<dyn System>,
  timers: HashMap<&'static str, SystemTimer>,
}

impl SystemContainer {
  pub fn new(system: Box<dyn System>) -> Self {
    Self {
      inner: system,
      timers: HashMap::new(),
    }
  }

  pub fn on_tick(&mut self, delta: &Duration, ctx: &mut Context) {
    let mut timers_to_remove = vec![];
    for (timer_name, timer) in &mut self.timers {
      if timer.tick(delta) {
        // Timer has completed, trigger a switch event with the timer's name
        self.inner.on_timer(timer_name, ctx);
        if let TimerMode::OneShot = timer.mode() {
          timers_to_remove.push(*timer_name);
        }
      }
    }

    for timer_name in timers_to_remove {
      self.timers.remove(timer_name);
    }
  }

  pub fn set_timer(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    self
      .timers
      .insert(timer_name, SystemTimer::new(duration, mode));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    self.timers.remove(timer_name);
  }
}

impl Deref for SystemContainer {
  type Target = dyn System;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

impl DerefMut for SystemContainer {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut *self.inner
  }
}
