use std::collections::{HashMap, HashSet};
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

  pub fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    let mut timers_to_remove = vec![];
    log::trace!(
      "SystemContainer tick: delta={:?}, timer count={}",
      delta,
      self.timers.len()
    );
    for (timer_name, timer) in &mut self.timers {
      if timer.tick(delta) {
        // Timer has completed, trigger a switch event with the timer's name
        log::trace!("Timer '{}' completed, triggering event", timer_name);
        let mut targets = HashSet::new();
        targets.insert(self.id);
        ctx.target(targets, TimerComplete::new(*timer_name));

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
