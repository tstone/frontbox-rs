//! The harness and systems shared by the run loop's tests.
//!
//! The harness drives a real [`RunLoop`] without tokio, ticks or hardware. Tests send it messages and
//! [`settle`](Harness::settle) it, which handles everything queued, and whatever that queues in turn, in the order the
//! loop would. What happened is then read from the loop's state, or from a [`Log`] that test systems write to.

use std::sync::{Arc, Mutex};

use super::*;

pub(super) struct Harness {
  run_loop: RunLoop,
  rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl std::ops::Deref for Harness {
  type Target = RunLoop;

  fn deref(&self) -> &RunLoop {
    &self.run_loop
  }
}

impl std::ops::DerefMut for Harness {
  fn deref_mut(&mut self) -> &mut RunLoop {
    &mut self.run_loop
  }
}

impl Harness {
  pub(super) fn new() -> Self {
    Self::with_state(AppState::default())
  }

  /// A harness over the given app state, e.g. one with switches
  pub(super) fn with_state(mut base: AppState) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    // as in `App::run`, so config changes emit `ConfigChanged`
    base.config.app_sender = Some(tx.clone());
    let run_loop = RunLoop::new(
      base,
      tx,
      TracerSenders::new(Vec::new()),
      Routines::new(Default::default()),
      0,
    );
    Self { run_loop, rx }
  }

  /// Queue a message, as a system or the machine would. Nothing happens until the harness settles.
  pub(super) fn send(&self, message: AppMessage) {
    let _ = self.app_sender.send(message);
  }

  /// Handle every queued message, and whatever handling them queues, until nothing is left
  pub(super) fn settle(&mut self) {
    for _ in 0..10_000 {
      let Ok(message) = self.rx.try_recv() else {
        return;
      };
      if self.run_loop.handle(message).is_break() {
        panic!("Settled a shutdown; test it by calling `handle` instead");
      }
    }
    panic!("The run loop never settled: handling messages keeps queueing more");
  }

  /// Broadcast the event, as the machine or a system would, and settle
  pub(super) fn emit(&mut self, event: impl Event) {
    self.send(AppMessage::EmitEvent(None, EventBox::new(event)));
    self.settle();
  }

  /// Tick, then settle
  pub(super) fn tick_by(&mut self, delta: Duration) {
    self.run_loop.tick(delta);
    self.settle();
  }

  /// Spawn the system into the root group, then settle
  pub(super) fn spawn(&mut self, system: impl Into<SystemContainer>) -> SystemHandle {
    self.spawn_in(ROOT_GROUP, system)
  }

  /// Spawn the system into the group, then settle. Panics if it couldn't be spawned.
  pub(super) fn spawn_in(&mut self, group: GroupId, system: impl Into<SystemContainer>) -> SystemHandle {
    let system = system.into();
    let (handle, name) = (SystemHandle::new(system.id(), group), system.name());
    self.spawn_system(system, group);
    assert!(self.is_running(handle), "{name} wasn't spawned");
    self.settle();
    handle
  }

  /// Spawn an active group owned by `owner`, holding one [`Member`], then settle
  pub(super) fn spawn_group(&mut self, owner: SystemHandle, name: &'static str) -> GroupId {
    self.spawn_group_of(owner, name, vec![Member.into()], true)
  }

  /// Spawn a group owned by `owner`, then settle
  pub(super) fn spawn_group_of(
    &mut self,
    owner: SystemHandle,
    name: &'static str,
    members: Vec<ChildSystemContainer>,
    active: bool,
  ) -> GroupId {
    let group = GroupHandle::new(GroupId::next(), name);
    let id = group.id();
    self.spawn_system_group(owner, group, members, active);
    self.settle();
    id
  }

  /// Spawn a [`Listener`] into the root group, returning what it hears from then on
  pub(super) fn listen(&mut self) -> Log<Heard> {
    let heard = Log::default();
    self.spawn(Listener(heard.clone()));
    heard.take(); // its own spawn
    heard
  }

  pub(super) fn is_running(&self, handle: SystemHandle) -> bool {
    self.groups.contains_system(handle)
  }
}

/// What test systems did or heard, in order. Shared by the systems and the test, and `Send` so the systems can also be
/// spawned by message.
pub(super) struct Log<T>(Arc<Mutex<Vec<T>>>);

impl<T> Log<T> {
  pub(super) fn push(&self, entry: T) {
    self.0.lock().unwrap().push(entry);
  }

  /// Everything logged since the last `take`
  pub(super) fn take(&self) -> Vec<T> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

impl<T> Clone for Log<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> Default for Log<T> {
  fn default() -> Self {
    Self(Arc::default())
  }
}

/// An event a [`Listener`] heard, for the events the run loop itself emits and those the tests send
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Heard {
  Spawned(u64),
  Despawned(u64),
  Deactivated(u64),
  Reactivated(u64),
  SwitchClosed(&'static str),
  SwitchOpened(&'static str),
  ConfigChanged(&'static str),
  Poke,
  Other,
}

impl Heard {
  fn of(event: &dyn Event) -> Self {
    if let Some(e) = event.downcast_ref::<SystemSpawned>() {
      Self::Spawned(e.id)
    } else if let Some(e) = event.downcast_ref::<SystemDespawned>() {
      Self::Despawned(e.id)
    } else if let Some(e) = event.downcast_ref::<SystemDeactivated>() {
      Self::Deactivated(e.id)
    } else if let Some(e) = event.downcast_ref::<SystemReactivated>() {
      Self::Reactivated(e.id)
    } else if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      Self::SwitchClosed(e.switch.name)
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      Self::SwitchOpened(e.switch.name)
    } else if let Some(e) = event.downcast_ref::<ConfigChanged>() {
      Self::ConfigChanged(e.0)
    } else if event.is::<Poke>() {
      Self::Poke
    } else {
      Self::Other
    }
  }
}

/// Records every event it hears
#[derive(Clone)]
pub(super) struct Listener(pub(super) Log<Heard>);

impl System for Listener {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    self.0.push(Heard::of(event));
  }
}

/// Logs each of its handlers as it's called, as `"<N> <handler>"`, and `"<N> heard Poke"` for each `Poke`. Numbered so
/// several can run in one group.
#[derive(Clone)]
pub(super) struct Probe<const N: usize>(pub(super) Log<String>);

impl<const N: usize> Probe<N> {
  fn log(&self, what: &str) {
    self.0.push(format!("{N} {what}"));
  }
}

impl<const N: usize> System for Probe<N> {
  fn on_spawn(&mut self, _ctx: &SystemContext) {
    self.log("on_spawn");
  }

  fn on_despawn(&mut self, _ctx: &SystemContext) {
    self.log("on_despawn");
  }

  fn on_deactivate(&mut self, _ctx: &SystemContext) {
    self.log("on_deactivate");
  }

  fn on_reactivate(&mut self, _ctx: &SystemContext) {
    self.log("on_reactivate");
  }

  fn on_tick(&mut self, _delta: Duration, _ctx: &SystemContext) {
    self.log("on_tick");
  }

  fn on_render(&mut self, _ctx: &SystemContext) {
    self.log("on_render");
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if event.is::<Poke>() {
      self.log("heard Poke");
    }
  }
}

/// Owns groups. Numbered so several can run in one group.
#[derive(Clone)]
pub(super) struct Owner<const N: usize>;

impl<const N: usize> System for Owner<N> {}

/// Fills a group
#[derive(Clone)]
pub(super) struct Member;

impl System for Member {}

/// An event with no meaning, for tests to send
#[derive(serde::Serialize, Event, Clone)]
pub(super) struct Poke;
