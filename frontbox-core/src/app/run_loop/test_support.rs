//! The harness and systems shared by the run loop's tests

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
    let (tx, rx) = mpsc::unbounded_channel();
    let run_loop = RunLoop::new(
      AppState::default(),
      tx,
      TracerSenders::new(Vec::new()),
      Routines::new(Default::default()),
      0,
    );
    Self { run_loop, rx }
  }

  /// The `SystemDeactivated` and `SystemReactivated` events sent so far, as `(system id, active)`
  pub(super) fn activity_events(&mut self) -> Vec<(u64, bool)> {
    let mut events = Vec::new();
    while let Ok(message) = self.rx.try_recv() {
      if let AppMessage::EmitEvent(_, event_box) = message {
        let event = event_box.event.as_ref();
        if let Some(e) = event.downcast_ref::<SystemDeactivated>() {
          events.push((e.id, false));
        } else if let Some(e) = event.downcast_ref::<SystemReactivated>() {
          events.push((e.id, true));
        }
      }
    }
    events
  }

  pub(super) fn spawn_owner(&mut self) -> SystemHandle {
    self.spawn(Owner)
  }

  pub(super) fn spawn_group(&mut self, owner: SystemHandle, name: &'static str) -> GroupId {
    let group = GroupHandle::new(GroupId::next(), name);
    let id = group.id();
    self.spawn_system_group(owner, group, vec![Member.into()], true);
    id
  }

  pub(super) fn despawn(&mut self, handle: SystemHandle) -> Vec<SystemHandle> {
    self.despawn_system(handle)
  }
}

impl Harness {
  pub(super) fn spawn(&mut self, system: impl Into<SystemContainer>) -> SystemHandle {
    let container = system.into();
    let handle = SystemHandle::new(container.id(), ROOT_GROUP);
    self.spawn_system(container, ROOT_GROUP);
    handle
  }

  pub(super) fn poke(&self) {
    self.emit_event(None, EventBox::new(Poke));
  }

  /// The handles of the `DespawnSystem` messages sent so far
  pub(super) fn despawns_requested(&mut self) -> Vec<SystemHandle> {
    let mut handles = Vec::new();
    while let Ok(message) = self.rx.try_recv() {
      if let AppMessage::DespawnSystem(handle) = message {
        handles.push(handle);
      }
    }
    handles
  }
}

#[derive(Clone)]
struct Owner;

impl System for Owner {}

#[derive(Clone)]
struct Member;

impl System for Member {}

#[derive(serde::Serialize, Event)]
pub(super) struct Poke;
