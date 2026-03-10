use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use crate::prelude::*;
use crate::systems::*;

pub struct SystemContainer {
  pub(crate) id: u64,
  pub(crate) inner: Box<dyn System>,
  timers: HashMap<&'static str, SystemTimer>,
}

impl SystemContainer {
  pub fn new(id: u64, system: Box<dyn System>) -> Self {
    Self {
      id,
      inner: system,
      timers: HashMap::new(),
    }
  }

  pub fn new_from_system(system: Box<dyn System>) -> Self {
    Self::new(next_listener_id(), system)
  }

  pub fn on_tick(&mut self, delta: Duration, ctx: &Context, cmds: &mut Commands) {
    let mut timers_to_remove = vec![];
    log::trace!(
      "SystemContainer tick: delta={:?}, timer count={}",
      delta,
      self.timers.len()
    );
    for (timer_name, timer) in &mut self.timers {
      if timer.tick(delta) {
        log::trace!("Timer '{}' completed, triggering event", timer_name);
        self.inner.on_timer(timer_name, ctx, cmds);
        if let TimerMode::OneShot = timer.mode() {
          timers_to_remove.push(*timer_name);
        }
      }
    }

    for timer_name in timers_to_remove {
      self.timers.remove(timer_name);
    }

    // bubble tick to inner system after processing timers
    self.inner.on_tick(delta, ctx, cmds);
  }

  pub fn set_timer(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    log::debug!(
      "Setting timer '{}' for {:?} with mode {:?}",
      timer_name,
      duration,
      mode
    );
    self
      .timers
      .insert(timer_name, SystemTimer::new(duration, mode));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    log::debug!("Clearing timer '{}'", timer_name);
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
