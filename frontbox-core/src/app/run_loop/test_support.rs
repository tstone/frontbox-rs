//! The systems shared by the run loop's tests, and helpers for driving them through the [`Harness`]. Test systems
//! write what they did or heard to a [`Log`] for the test to read.

use std::sync::{Arc, Mutex};

use super::*;

pub(super) use super::harness::Harness;

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

/// Helpers for the systems above
impl Harness {
  /// Spawn an active group owned by `owner`, holding one [`Member`], then settle
  pub(super) fn spawn_group(&mut self, owner: SystemHandle, name: &'static str) -> GroupId {
    self.spawn_group_of(owner, name, vec![Member.into()], true)
  }

  /// Spawn a [`Listener`] into the root group, returning what it hears from then on
  pub(super) fn listen(&mut self) -> Log<Heard> {
    let heard = Log::default();
    self.spawn(Listener(heard.clone()));
    heard.take(); // its own spawn
    heard
  }
}
