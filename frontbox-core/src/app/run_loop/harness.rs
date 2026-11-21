//! A harness that drives a real [`RunLoop`] without tokio, ticks or hardware. Tests send it messages and
//! [`settle`](Harness::settle) it, which handles everything queued, and whatever that queues in turn, in the order the
//! loop would. What happened is then read from the loop's state.
//!
//! Built for the run loop's own tests and, with the `testing` feature, for test support built on it.

use super::*;

pub(crate) struct Harness {
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
  #[cfg(test)]
  pub(crate) fn new() -> Self {
    Self::with_state(AppState::default())
  }

  /// A harness whose traces are sent to the returned receiver, as an attached tracer gets them
  #[cfg(test)]
  pub(crate) fn tracing() -> (Self, mpsc::UnboundedReceiver<TraceEvent>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut harness = Self::new();
    harness.run_loop.tracers = TracerSenders::new(vec![tx]);
    harness.run_loop.base.traced = true;
    (harness, rx)
  }

  /// A harness over the given app state, e.g. one with switches
  #[cfg(test)]
  pub(crate) fn with_state(base: AppState) -> Self {
    Self::with(
      base,
      Routines::new(Default::default()),
      0,
      mpsc::unbounded_channel(),
    )
  }

  /// A harness over the given app state and routines, as `App::run` sets them up. `machine_system_id` is the system
  /// switch changes are reported as coming from, and `channel` is the one the loop reads, whose sender systems get.
  pub(crate) fn with(
    mut base: AppState,
    routines: Routines,
    machine_system_id: u64,
    channel: (
      mpsc::UnboundedSender<AppMessage>,
      mpsc::UnboundedReceiver<AppMessage>,
    ),
  ) -> Self {
    let (tx, rx) = channel;
    // as in `App::run`, so config changes emit `ConfigChanged`
    base.config.app_sender = Some(tx.clone());
    let run_loop = RunLoop::new(
      base,
      tx,
      TracerSenders::new(Vec::new()),
      routines,
      machine_system_id,
    );
    Self { run_loop, rx }
  }

  /// Queue a message, as a system or the machine would. Nothing happens until the harness settles.
  pub(crate) fn send(&self, message: AppMessage) {
    let _ = self.app_sender.send(message);
  }

  /// Handle every queued message, and whatever handling them queues, until nothing is left
  pub(crate) fn settle(&mut self) {
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
  pub(crate) fn emit(&mut self, event: impl Event) {
    self.send(AppMessage::EmitEvent(None, EventBox::new(event)));
    self.settle();
  }

  /// Tick, then settle
  pub(crate) fn tick_by(&mut self, delta: Duration) {
    self.run_loop.tick(delta);
    self.settle();
  }

  /// Spawn the system at the top level, then settle
  pub(crate) fn spawn(&mut self, system: impl Into<SystemContainer>) -> SystemHandle {
    self.spawn_under(None, system)
  }

  /// Spawn the system as a child of `parent`, or at the top level, then settle. Panics if it couldn't be spawned.
  pub(crate) fn spawn_under(
    &mut self,
    parent: Option<SystemHandle>,
    system: impl Into<SystemContainer>,
  ) -> SystemHandle {
    let system = system.into();
    let name = system.name();
    let handle = self
      .spawn_system(system, parent)
      .unwrap_or_else(|| panic!("{name} wasn't spawned"));
    self.settle();
    handle
  }

  /// Spawn the systems at the top level, then settle, as `RunLoop::run` does at boot: whatever one queues on spawn is
  /// handled once they're all running
  pub(crate) fn spawn_initial(&mut self, systems: impl IntoIterator<Item = SystemContainer>) {
    for system in systems {
      self.spawn_system(system, None);
    }
    self.settle();
  }

  /// Spawn a [`Group`] of `members` as a child of `owner`, deactivated unless `active`, then settle. Returns the
  /// group's handle.
  #[cfg(test)]
  pub(crate) fn spawn_group_of(
    &mut self,
    owner: SystemHandle,
    name: &'static str,
    members: Vec<ChildSystemContainer>,
    active: bool,
  ) -> SystemHandle {
    let group = self.spawn_under(Some(owner), Group::new(name, members));
    if !active {
      self.send(AppMessage::DeactivateChild(owner, group.id));
      self.settle();
    }
    group
  }

  /// The handles of the system's children, in spawn order
  #[cfg(test)]
  pub(crate) fn children_of(&self, parent: SystemHandle) -> Vec<SystemHandle> {
    self
      .groups
      .children_group(parent)
      .map(|group| self.groups.handles_in(group))
      .unwrap_or_default()
  }

  /// A context over the loop's state, for calling into systems from outside the loop. What it sends is handled on the
  /// next settle.
  pub(crate) fn service_context(&self) -> ServiceContext<'_> {
    ServiceContext::new(&self.base, &self.groups, &self.app_sender)
  }

  /// How often the loop ticks
  pub(crate) fn tick_interval(&self) -> Duration {
    self.base.system_interval
  }

  pub(crate) fn is_running(&self, handle: SystemHandle) -> bool {
    self.groups.contains_system(handle)
  }
}
