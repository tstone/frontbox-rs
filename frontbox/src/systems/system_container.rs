use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use crate::animation::Accumulator;
use crate::prelude::Signal;
use crate::systems::*;

static INCR_ID: AtomicU64 = AtomicU64::new(0);

pub struct SystemContainer {
  pub id: u64,
  pub(crate) inner: Box<dyn System>,
  cues: HashMap<u64, CueAccumulator>,
  last_active_state: bool,
}

impl SystemContainer {
  pub fn new(id: u64, system: Box<dyn System>) -> Self {
    Self {
      id,
      inner: system,
      cues: HashMap::new(),
      last_active_state: true,
    }
  }

  pub(crate) fn next_id() -> u64 {
    INCR_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  }

  pub fn new_from_system(system: Box<dyn System>) -> Self {
    Self::new(SystemContainer::next_id(), system)
  }

  /// Checks if the system is active and fires reactivate/deactivate handlers if it has changed since the last check.
  /// Use `is_active` if you just want to check the active state without firing handlers.
  pub fn handle_active(&mut self, ctx: &mut Context) -> bool {
    let fresh = self.inner.is_active(ctx);

    if fresh != self.last_active_state {
      if fresh {
        // system just became active
        self.inner.on_reactivate(ctx);
      } else {
        // system just became inactive
        self.inner.on_deactivate(ctx);
      }
    }

    self.last_active_state = fresh;
    fresh
  }

  pub fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    let mut cues_to_remove = vec![];
    for (id, cue) in self.cues.iter_mut() {
      if cue.accumulate(delta).completed_cycle {
        log::trace!("Cue {} cycle completed, triggering signal", id);
        if let Some(signal) = cue.signal() {
          self.inner.on_cue(signal, ctx);
        }

        if cue.is_complete() {
          log::trace!("Cue {} is entirely completed, removing", id);
          cues_to_remove.push(*id);
        }
      }
    }

    for id in cues_to_remove {
      self.cues.remove(&id);
    }

    // bubble tick to inner system after processing timers
    self.inner.on_tick(delta, ctx);
  }

  pub fn create_cue(&mut self, cue: Cue, id: u64, signals: Vec<Box<dyn Signal>>) {
    self.cues.insert(id, CueAccumulator::new(cue, signals));
  }

  pub fn cancel_cue(&mut self, cue_id: u64) {
    self.cues.remove(&cue_id);
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
