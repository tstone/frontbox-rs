use std::any::{Any, TypeId, type_name_of_val};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use crate::animation::Accumulator;
use crate::prelude::Signal;
use crate::systems::*;

static INCR_ID: AtomicU64 = AtomicU64::new(0);

pub struct SystemContainer {
  id: u64,
  name: String,
  inner: Box<dyn System>,
  cues: HashMap<u64, CueAccumulator>,
  last_active_state: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
}

impl SystemContainer {
  pub fn new<T: System + 'static>(system: T) -> Self {
    let name = type_name_of_val(&system).to_string();
    Self {
      id: Self::next_id(),
      name,
      inner: Box::new(system),
      cues: HashMap::new(),
      last_active_state: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn type_id(&self) -> TypeId {
    (self.as_any)(self.inner.as_ref()).type_id()
  }

  pub(crate) fn next_id() -> u64 {
    INCR_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  }

  /// Checks if the system is active and fires reactivate/deactivate handlers if it has changed since the last check.
  /// Use `is_active` if you just want to check the active state without firing handlers.
  pub(crate) fn handle_active(&mut self, ctx: &mut Context) -> bool {
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

  pub(crate) fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
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

  pub(crate) fn create_cue(&mut self, cue: Cue, id: u64, signals: Vec<Box<dyn Signal>>) {
    self.cues.insert(id, CueAccumulator::new(cue, signals));
  }

  pub(crate) fn cancel_cue(&mut self, cue_id: u64) {
    self.cues.remove(&cue_id);
  }

  pub(crate) fn downcast_ref<T: System + 'static>(&self) -> Option<&T> {
    (self.as_any)(self.inner.as_ref()).downcast_ref::<T>()
  }
  pub(crate) fn downcast_mut<T: System + 'static>(&mut self) -> Option<&mut T> {
    (self.as_any_mut)(self.inner.as_mut()).downcast_mut::<T>()
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

impl<T: System + 'static> From<T> for SystemContainer {
  fn from(system: T) -> Self {
    Self::new(system)
  }
}

pub struct SpawnableSystemContainer {
  id: u64,
  name: String,
  inner: Box<dyn SpawnableSystem>,
  cues: HashMap<u64, CueAccumulator>,
  last_active_state: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
}

impl SpawnableSystemContainer {
  pub fn new<T: SpawnableSystem + 'static>(system: T) -> Self {
    let name = type_name_of_val(&system).to_string();
    Self {
      id: SystemContainer::next_id(),
      name,
      inner: Box::new(system),
      cues: HashMap::new(),
      last_active_state: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
    }
  }

  pub fn to_system_container(self) -> SystemContainer {
    SystemContainer {
      id: self.id,
      name: self.name,
      inner: self.inner,
      cues: self.cues,
      last_active_state: self.last_active_state,
      as_any: self.as_any,
      as_any_mut: self.as_any_mut,
    }
  }
}

impl<T: SpawnableSystem + 'static> From<T> for SpawnableSystemContainer {
  fn from(system: T) -> Self {
    Self::new(system)
  }
}

#[derive(Clone)]
pub struct ChildSystemContainer {
  id: u64,
  name: String,
  inner: Box<dyn ChildSystem>,
  cues: HashMap<u64, CueAccumulator>,
  last_active_state: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
}

impl ChildSystemContainer {
  pub fn new<T: ChildSystem + 'static>(system: T) -> Self {
    let name = type_name_of_val(&system).to_string();
    Self {
      id: SystemContainer::next_id(),
      name,
      inner: Box::new(system),
      cues: HashMap::new(),
      last_active_state: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
    }
  }

  pub fn to_system_container(self) -> SystemContainer {
    SystemContainer {
      id: self.id,
      name: self.name,
      inner: self.inner,
      cues: self.cues,
      last_active_state: self.last_active_state,
      as_any: self.as_any,
      as_any_mut: self.as_any_mut,
    }
  }
}

impl<T: ChildSystem + 'static> From<T> for ChildSystemContainer {
  fn from(system: T) -> Self {
    Self::new(system)
  }
}
