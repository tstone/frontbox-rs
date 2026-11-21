use std::any::{Any, TypeId, type_name_of_val};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use crate::app::app_tracer::{ActivityCause, TraceEvent};
use crate::app::run_loop::current_delivery;
use crate::prelude::Event;
use crate::prelude::app_tracer::TracerSenders;
use crate::systems::*;

// from 1, so a default `SystemHandle` (id 0) never refers to a system
static INCR_ID: AtomicU64 = AtomicU64::new(1);

pub struct SystemContainer {
  id: u64,
  name: &'static str,
  description: Option<&'static str>,
  inner: Box<dyn System>,
  timers: HashMap<u64, TimerAccumulator>,
  last_active_state: bool,
  faulted: bool,
  /// Whether its parent lets it run (`ctx.systems.activate` and `deactivate`). Always true for a top-level system.
  enabled: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
  provided: ProvidedTraits,
}

impl SystemContainer {
  pub fn new<T: System + 'static>(system: T) -> Self {
    Self {
      id: Self::next_id(),
      name: type_name_of_val(&system),
      description: T::description(),
      inner: Box::new(system),
      timers: HashMap::new(),
      last_active_state: true,
      faulted: false,
      enabled: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
      provided: Provides::<T>::collect(),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn name(&self) -> &'static str {
    &self.name
  }

  /// Name it for display, in place of its type's name (e.g. a `Group` by what it holds)
  pub(crate) fn set_name(&mut self, name: &'static str) {
    self.name = name;
  }

  /// What the system is for, from [`System::description`], for the console
  pub fn description(&self) -> Option<&'static str> {
    self.description
  }

  /// Describe it, in place of its type's description (e.g. a `Group` by what it holds)
  pub(crate) fn set_description(&mut self, description: &'static str) {
    self.description = Some(description);
  }

  pub fn type_id(&self) -> TypeId {
    (self.as_any)(self.inner.as_ref()).type_id()
  }

  /// Whether one of the system's handlers has panicked
  pub(crate) fn is_faulted(&self) -> bool {
    self.faulted
  }

  /// Calls `f`, catching a panic so it can't unwind out of the run loop and end the process. A panic is logged and
  /// returns `None`. Use [`retire_on_panic`](Self::retire_on_panic) unless the system is already on its way out.
  ///
  /// The system may be left half-updated by the panic, which is why `retire_on_panic` retires it. Anything else `f`
  /// borrowed is released as the panic unwinds; `RefCell`s aren't poisoned.
  pub(crate) fn catch_panic<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(|| f(self))) {
      Ok(result) => Some(result),
      Err(payload) => {
        log::error!(
          target: "frontbox::systems",
          "💥 System {} ({}) panicked: {}",
          self.name(),
          self.id(),
          panic_message(payload.as_ref())
        );
        None
      }
    }
  }

  /// [`catch_panic`](Self::catch_panic), and if `f` panics, retire the system: mark it faulted so nothing more is
  /// handed to it, and queue its despawn. `ctx` must be this system's context.
  pub(crate) fn retire_on_panic<T>(
    &mut self,
    ctx: &SystemContext,
    f: impl FnOnce(&mut Self) -> T,
  ) -> Option<T> {
    let result = self.catch_panic(f);
    if result.is_none() && !self.faulted {
      log::error!(
        target: "frontbox::systems",
        "💥 Despawning system {} ({}) after it panicked",
        self.name(),
        self.id()
      );
      self.faulted = true;
      ctx.systems.despawn_self();
    }
    result
  }

  pub(crate) fn next_id() -> u64 {
    INCR_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  }

  /// Called when the system is deactivated, by its own `is_active`, its parent, or an ancestor stopping. The run loop
  /// broadcasts [`SystemDeactivated`] afterwards.
  pub(crate) fn on_deactivate(&mut self, ctx: &SystemContext) {
    log::debug!(
      target: "frontbox::systems",
      "🌐 System {} ({}) is now inactive",
      self.name(),
      self.id()
    );
    self.inner.on_deactivate(ctx);
  }

  /// Called when the system is re-activated after being deactivated. The run loop broadcasts [`SystemReactivated`]
  /// afterwards.
  pub(crate) fn on_reactivate(&mut self, ctx: &SystemContext) {
    log::debug!(
      target: "frontbox::systems",
      "🌐 System {} ({}) is now active",
      self.name(),
      self.id()
    );
    self.inner.on_reactivate(ctx);
  }

  /// Prepare the system to run, and fire initial lifecycle events. `parent_active` is whether its parent is active,
  /// so it can be.
  pub(crate) fn initialize(&mut self, ctx: &SystemContext, parent_active: bool) {
    self.last_active_state = parent_active && self.enabled && self.inner.is_active(&ctx);
    self.on_spawn(ctx);
  }

  /// Whether the system was active when last checked
  pub(crate) fn was_active(&self) -> bool {
    self.last_active_state
  }

  /// Let the system run, or stop it, as its parent asks. Takes effect at the next check of its activity.
  pub(crate) fn set_enabled(&mut self, enabled: bool) {
    self.enabled = enabled;
  }

  /// Checks if the system is active and fires reactivate/deactivate handlers if it has changed since the last check.
  /// Only called while its parent is active. Use `is_active` if you just want to check the active state without firing
  /// handlers.
  pub(crate) fn handle_active(
    &mut self,
    ctx: &SystemContext,
    tracer_txs: &TracerSenders,
  ) -> ActivityCheck {
    // its parent is active and lets it run, as before, so only its own `is_active` can have changed
    let cause = ActivityCause::IsActive {
      during: current_delivery(),
    };
    self.check_activity(true, cause, ctx, tracer_txs)
  }

  /// [`handle_active`](Self::handle_active), for when its parent's activity has changed to `parent_active`, or its
  /// parent has activated or deactivated it. `cause` is what to trace if its activity changes.
  pub(crate) fn check_activity(
    &mut self,
    parent_active: bool,
    cause: ActivityCause,
    ctx: &SystemContext,
    tracer_txs: &TracerSenders,
  ) -> ActivityCheck {
    let fresh = parent_active && self.enabled && self.inner.is_active(ctx);

    if fresh != self.last_active_state {
      if fresh {
        self.on_reactivate(ctx);
      } else {
        self.on_deactivate(ctx);
      }
      tracer_txs.send(TraceEvent::SystemActiveStateChange {
        id: self.id,
        active: fresh,
        cause,
      });
      // so routines waiting on this system look again
      ctx.routines.report_activity_changed();
    }

    let changed = fresh != self.last_active_state;
    self.last_active_state = fresh;
    ActivityCheck {
      active: fresh,
      changed,
    }
  }

  pub(crate) fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    let system_id = self.id;
    self.timers.retain(|id, timer| {
      timer.accumulate(delta, |signal| {
        log::trace!(target: "frontbox::timers", "Timer {} @ system {} fired", id, system_id);
        self.inner.on_event(signal, ctx);
      });

      if timer.is_complete() {
        log::trace!(target: "frontbox::timers", "Timer {} @ system {} completed, removing", id, system_id);
      }
      !timer.is_complete()
    });

    // bubble tick to inner system after processing timers
    self.inner.on_tick(delta, ctx);
  }

  pub(crate) fn create_timer(&mut self, timer: Timer, id: u64, signals: Vec<Box<dyn Event>>) {
    self
      .timers
      .insert(id, TimerAccumulator::new(timer, signals));
  }

  pub(crate) fn cancel_timer(&mut self, timer_id: u64) {
    self.timers.remove(&timer_id);
  }

  /// Does nothing for a timer that has completed or was canceled
  pub(crate) fn set_timer_paused(&mut self, timer_id: u64, paused: bool) {
    if let Some(timer) = self.timers.get_mut(&timer_id) {
      timer.set_paused(paused);
    }
  }

  #[allow(unused)]
  pub(crate) fn downcast_ref<T: System + 'static>(&self) -> Option<&T> {
    (self.as_any)(self.inner.as_ref()).downcast_ref::<T>()
  }

  pub(crate) fn downcast_mut<T: System + 'static>(&mut self) -> Option<&mut T> {
    (self.as_any_mut)(self.inner.as_mut()).downcast_mut::<T>()
  }

  /// The traits the system can be looked up by, besides its own type. See [`System::provides`].
  pub(crate) fn provided_traits(&self) -> impl Iterator<Item = TypeId> + '_ {
    self.provided.type_ids()
  }

  /// The system as the trait `I`, if it provides it
  pub(crate) fn cast_mut<I: ?Sized + 'static>(&mut self) -> Option<&mut I> {
    self.provided.cast_mut::<I>(self.inner.as_mut())
  }
}

/// The result of [`SystemContainer::handle_active`]
pub(crate) struct ActivityCheck {
  pub active: bool,
  /// Whether `active` differs from the last check, so a deactivate or reactivate handler ran
  pub changed: bool,
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
  name: &'static str,
  description: Option<&'static str>,
  inner: Box<dyn SpawnableSystem>,
  timers: HashMap<u64, TimerAccumulator>,
  last_active_state: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
  provided: ProvidedTraits,
}

impl SpawnableSystemContainer {
  pub fn new<T: SpawnableSystem + 'static>(system: T) -> Self {
    Self {
      id: SystemContainer::next_id(),
      name: type_name_of_val(&system),
      description: T::description(),
      inner: Box::new(system),
      timers: HashMap::new(),
      last_active_state: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
      provided: Provides::<T>::collect(),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn to_system_container(self) -> SystemContainer {
    SystemContainer {
      id: self.id,
      name: self.name,
      description: self.description,
      inner: self.inner,
      timers: self.timers,
      last_active_state: self.last_active_state,
      faulted: false,
      enabled: true,
      as_any: self.as_any,
      as_any_mut: self.as_any_mut,
      provided: self.provided,
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
  name: &'static str,
  description: Option<&'static str>,
  inner: Box<dyn ChildSystem>,
  timers: HashMap<u64, TimerAccumulator>,
  last_active_state: bool,
  as_any: fn(&dyn System) -> &dyn Any,
  as_any_mut: fn(&mut dyn System) -> &mut dyn Any,
  provided: ProvidedTraits,
}

impl ChildSystemContainer {
  pub fn new<T: ChildSystem + 'static>(system: T) -> Self {
    Self {
      name: type_name_of_val(&system),
      description: T::description(),
      inner: Box::new(system),
      timers: HashMap::new(),
      last_active_state: true,
      as_any: |s| s as &dyn Any,
      as_any_mut: |s| s as &mut dyn Any,
      provided: Provides::<T>::collect(),
    }
  }

  /// The system, ready to spawn. Each one gets a new id, since a template is copied once for every player.
  pub fn to_system_container(self) -> SystemContainer {
    SystemContainer {
      id: SystemContainer::next_id(),
      name: self.name,
      description: self.description,
      inner: self.inner,
      timers: self.timers,
      last_active_state: self.last_active_state,
      faulted: false,
      enabled: true,
      as_any: self.as_any,
      as_any_mut: self.as_any_mut,
      provided: self.provided,
    }
  }
}

impl<T: ChildSystem + 'static> From<T> for ChildSystemContainer {
  fn from(system: T) -> Self {
    Self::new(system)
  }
}

/// The message a panic was raised with, when it's a string (as `panic!` and `expect` make it)
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> &str {
  payload
    .downcast_ref::<&str>()
    .copied()
    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
    .unwrap_or("<non-string panic payload>")
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;
  use std::rc::Rc;

  use super::*;

  /// Active while its flag is set
  struct Toggled(Rc<Cell<bool>>);

  impl System for Toggled {
    fn is_active(&self, _ctx: &SystemContext) -> bool {
      self.0.get()
    }
  }

  #[test]
  fn handle_active_reports_changes() {
    let ctx = TestContext::default();
    let tracers = TracerSenders::new(Vec::new());
    let active = Rc::new(Cell::new(true));
    let mut system = SystemContainer::new(Toggled(active.clone()));
    system.initialize(&ctx.sys_ctx(), true);
    let mut check = || {
      let check = system.handle_active(&ctx.sys_ctx(), &tracers);
      (check.active, check.changed)
    };

    assert_eq!(check(), (true, false));
    active.set(false);
    assert_eq!(check(), (false, true));
    assert_eq!(check(), (false, false));
    active.set(true);
    assert_eq!(check(), (true, true));
  }
}
