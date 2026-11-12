use fast_protocol::SwitchState;
use std::any::{Any, TypeId};
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::Command;
use std::sync::Arc;

use tokio::sync::{Notify, mpsc, watch};
use tokio::time::{Duration, Instant};

use crate::app::GroupsExt;
use crate::app::app_message::AppMessage::EmitEvent;
use crate::app::app_tracer::{AppTracer, TraceEvent};
use crate::metrics;
use crate::prelude::app_message::*;
use crate::prelude::app_tracer::*;
use crate::prelude::*;
use crate::systems::SystemContainer;
use crate::systems::routine::Routines;
use crate::systems::state::{clear_states_owned_by, set_state};

/// The app's main loop. It owns the app state and every system, and handles the messages, ticks and tracer commands
/// that drive them, one at a time on a single task.
pub(crate) struct RunLoop {
  base: AppState,
  groups: Groups,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracers: TracerSenders,
  /// Notified on [`Synchronize`] to restart the tick timer
  resync: Arc<Notify>,
  routines: Routines,
  machine_system_id: u64,
}

impl RunLoop {
  fn new(
    base: AppState,
    app_sender: mpsc::UnboundedSender<AppMessage>,
    tracers: TracerSenders,
    routines: Routines,
    machine_system_id: u64,
  ) -> Self {
    let mut groups = Groups::new();
    groups.insert(ROOT_GROUP, SystemGroup::new(ROOT_GROUP_NAME));
    tracers.send(TraceEvent::SystemGroupSpawned {
      id: ROOT_GROUP,
      name: ROOT_GROUP_NAME,
    });

    Self {
      base,
      groups,
      app_sender,
      tracers,
      resync: Arc::new(Notify::new()),
      routines,
      machine_system_id,
    }
  }

  /// Spawn the initial systems and run until shut down
  pub(crate) async fn run(
    base: AppState,
    initial_systems: Vec<SystemContainer>,
    app_tracers: Vec<Box<dyn AppTracer>>,
    app_sender: mpsc::UnboundedSender<AppMessage>,
    mut app_receiver: mpsc::UnboundedReceiver<AppMessage>,
    mut tracer_ctl_receiver: mpsc::UnboundedReceiver<TracerControlEvent>,
    machine_system_id: u64,
    routines: Routines,
  ) {
    let tracers = TracerSenders::new(app_tracers.iter().map(|tracer| tracer.sender()).collect());
    let mut run_loop = Self::new(base, app_sender.clone(), tracers, routines, machine_system_id);

    for system in initial_systems {
      run_loop.spawn_system(system, ROOT_GROUP);
    }

    let (tick_tx, mut tick_rx) = watch::channel(());
    spawn_system_ticker(run_loop.base.system_interval, tick_tx, run_loop.resync.clone());
    run_loop.trace_configs();

    // listen for ctrl-c to trigger shutdown
    tokio::spawn(async move {
      tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
      log::info!("Ctrl+C signal received.");
      let _ = app_sender.send(AppMessage::Shutdown(ShutdownScope::Process));
    });

    let mut metrics_flush = tokio::time::interval_at(
      tokio::time::Instant::now() + metrics::FLUSH_INTERVAL,
      metrics::FLUSH_INTERVAL,
    );

    log::info!(target: "frontbox_core::run_loop", "⟳ Run loop started.");
    let mut last_tick = Instant::now();

    let shutdown_scope = loop {
      tokio::select! {
        Some(message) = app_receiver.recv() => {
          log::trace!(target: "frontbox_core::run_loop", "Run loop: AppMessage queue depth: {}", app_receiver.len());
          log::trace!(target: "frontbox_core::run_loop", "Run loop: Now handling {:?}", message);
          let message_text = format!("{:?}", message);
          let start = std::time::Instant::now();

          let flow = run_loop.handle(message);

          let duration = start.elapsed().as_micros();
          if duration > 1000 {
            log::warn!(target: "frontbox_core::run_loop", "Run loop Took {}μ to process {}", duration, message_text);
          } else {
            log::trace!(target: "frontbox_core::run_loop", "Run loop: Took {}μ", duration);
          }

          if let ControlFlow::Break(scope) = flow {
            break scope;
          }
        }

        Ok(_) = tick_rx.changed() => {
          let now = Instant::now();
          let delta = tick_delta(now - last_tick, run_loop.base.system_interval);
          last_tick = now;
          run_loop.tick(delta);
        }

        _ = metrics_flush.tick() => {
          run_loop.base.metrics.flush();
        }

        Some(command) = tracer_ctl_receiver.recv() => {
          run_loop.handle_tracer_control(command);
        }
      }
    };

    run_loop.shutdown(shutdown_scope).await;
  }

  /// Handle one message. Breaks when the message asks for a shutdown.
  fn handle(&mut self, message: AppMessage) -> ControlFlow<ShutdownScope> {
    match message {
      AppMessage::EmitEvent(sender, event_box) => {
        self.emit_event(sender, event_box);
      }
      AppMessage::EmitEventOriginFirst(origin, event_box) => {
        emit_event_to_origin(origin, event_box, &self.groups, &self.base, &self.app_sender, &self.tracers);
      }
      AppMessage::EmitEventToOthers(handled_by, event_box) => {
        emit_event_to_others(handled_by, event_box, &self.groups, &self.base, &self.app_sender, &self.tracers);
      }
      AppMessage::SwitchStateChange(id, state) => {
        self.switch_state_changed(Some(self.machine_system_id), id, state);
      }
      AppMessage::SyncSwitchStates(switch_states) => {
        self.base.switches.update_switch_states(switch_states);
      }
      AppMessage::ConfigChanged(key) => {
        self.base.config.save();
        self.emit_event(None, EventBox::new(ConfigChanged(key)));
      }
      AppMessage::DriverModeChange(driver_id, mode) => {
        self.base.drivers.set_mode(driver_id, mode.clone());
        self.tracers.send(TraceEvent::DriverModeChange { driver_id, mode });
      }
      AppMessage::Shutdown(scope) => {
        log::warn!(target: "frontbox_core::run_loop", "⏹️ Shutdown command received, shutting down...");
        return ControlFlow::Break(scope);
      }
      AppMessage::SpawnSystem(group, system) => {
        self.spawn_system(system.to_system_container(), group);
      }
      // Routines hear about despawns and group activity here, after the change, rather than through a message of
      // their own, so they see it in queue order: a group deactivated and then reactivated still ends a step.
      AppMessage::ReplaceSystem(handle, system) => {
        let despawned = self.replace_system(handle, system.to_system_container());
        self.activation_changed(&despawned);
      }
      AppMessage::DespawnSystem(handle) => {
        let despawned = self.despawn_system(handle);
        self.activation_changed(&despawned);
      }
      AppMessage::SpawnSystemGroup(owner, group, child_systems, active) => {
        self.spawn_system_group(owner, group, child_systems, active);
      }
      AppMessage::DespawnSystemGroup(group) => {
        let despawned = self.despawn_system_group(group);
        self.activation_changed(&despawned);
      }
      AppMessage::ActivateSystemGroup(group) => {
        self.activate_system_group(group);
        self.activation_changed(&self.groups.handles_in(group));
      }
      AppMessage::DeactivateSystemGroup(group) => {
        self.deactivate_system_group(group);
        self.activation_changed(&self.groups.handles_in(group));
      }
      AppMessage::CreateCue(handle, cue_id, cue, signals) => {
        self.create_cue(handle, cue_id, cue, signals);
      }
      AppMessage::CancelCue(handle, cue_id) => {
        self.cancel_cue(handle, cue_id);
      }
      AppMessage::StartRoutine(start) => {
        let (origin, type_id) = (start.origin, start.type_id);
        if let Some(started) = self.routines.start(start, &self.groups, &self.tracers) {
          // emitted here and the first step queued after it, so whatever its listeners emit lands first
          self.emit_event(Some(origin.id), started);
          let _ = self.app_sender.send(AppMessage::BeginRoutine(type_id));
        }
      }
      AppMessage::BeginRoutine(type_id) => {
        self.routines.begin(type_id, &self.groups, &self.base, &self.app_sender, &self.tracers);
      }
      AppMessage::CompleteRoutineStep(handle, type_id) => {
        self.routines.complete_step(handle, type_id, &self.groups, &self.base, &self.app_sender, &self.tracers);
      }
      AppMessage::HaltRoutine(handle, type_id) => {
        self.routines.halt(handle, type_id, &self.groups, &self.base, &self.app_sender, &self.tracers);
      }
      AppMessage::SystemActivityChanged(handle) => {
        self.activation_changed(&[handle]);
      }
      AppMessage::SetState(state) => {
        let (owner, name, value) = (state.owner, state.name, (state.to_json)(state.value.as_ref()));
        if let Some(changed) = set_state(&mut self.groups, state) {
          self.tracers.send(TraceEvent::StateChange {
            group: owner.group,
            name,
            value: Some(value),
          });
          self.emit_event(Some(owner.id), changed);
        }
      }
      AppMessage::TracerEvent(event) => {
        self.tracers.send(event);
      }
    }

    ControlFlow::Continue(())
  }

  fn handle_tracer_control(&mut self, command: TracerControlEvent) {
    match command {
      TracerControlEvent::OpenSwitch { switch_id } => {
        if self.base.switches.is_closed_by_id(switch_id).unwrap_or(false) {
          log::warn!(target: "frontbox_core::run_loop", "Tracer forcibly opening switch {switch_id}...");
          self.switch_state_changed(None, switch_id, SwitchState::Open);
        }
      }
      TracerControlEvent::CloseSwitch { switch_id } => {
        if self.base.switches.is_open_by_id(switch_id).unwrap_or(false) {
          log::warn!(target: "frontbox_core::run_loop", "Tracer forcibly closing switch {switch_id}...");
          self.switch_state_changed(None, switch_id, SwitchState::Closed);
        }
      }
      TracerControlEvent::SetConfig { key, value } => {
        // saved, emitted and applied to hardware through `AppMessage::ConfigChanged`, like a menu edit
        match self.base.config.set_by_key(&key, value) {
          Ok(()) => log::info!(target: "frontbox_core::run_loop", "Tracer set config '{key}' to {value:?}"),
          Err(err) => log::warn!(target: "frontbox_core::run_loop", "Tracer couldn't set config: {err}"),
        }
      }
    }
  }

  fn tick(&self, delta: Duration) {
    // Tick first is where most systems should do any time-based processing
    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_tick(delta, ctx);
    });
    // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_render(ctx);
    });
  }

  /// Call `on_despawn` on every system, regardless of whether it was active, in reverse spawn order
  async fn shutdown(self, scope: ShutdownScope) {
    for (key, group) in self.groups.iter().rev() {
      for cell in group.values().rev() {
        let mut system = cell.borrow_mut();
        let ctx = SystemContext::new(
          &self.base,
          SystemHandle::new(system.id(), *key),
          &self.groups,
          &self.app_sender,
        );
        call_system(&mut system, "on_despawn", |system| system.on_despawn(&ctx));
      }
    }

    // after despawning, so anything systems record on their way out is kept
    self.base.metrics.close().await;

    // wait a sec to allow systems to process shutdown event and clear timers, etc.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    if scope == ShutdownScope::OperatingSystem {
      if let Err(e) = Command::new("sudo").args(["shutdown"]).status() {
        log::error!(target: "frontbox_core::run_loop", "Failed to invoke OS shutdown: {e}");
      }
    }
  }

  /// Tell routines that these systems were despawned or may have changed activity
  fn activation_changed(&mut self, handles: &[SystemHandle]) {
    self
      .routines
      .system_activation_changed(handles, &self.groups, &self.base, &self.app_sender, &self.tracers);
  }

  fn emit_event(&self, sender: Option<u64>, event_box: EventBox) {
    if event_box.type_id == SYNCHRONIZE_TYPE_ID {
      self.resync.notify_one();
    }

    // notify any monitoring tracers
    self.tracers.send(TraceEvent::Event {
      type_name: event_box.type_name,
      event: event_box.try_json(),
      sender,
    });

    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_event(event_box.event.as_ref(), ctx);
    });
  }

  /// Tells tracers every config in use. Hardware configs are all known at boot, so they're listed even if nothing has
  /// read them yet; software configs join as they're first used.
  fn trace_configs(&self) {
    for (_, mode) in self.base.drivers.modes() {
      for config in mode.configs() {
        self.base.config.inspect(config);
      }
    }
    self.tracers.send(TraceEvent::ConfigsInUse {
      configs: self.base.config.views(),
    });
  }

  fn switch_state_changed(&mut self, sender: Option<u64>, switch_id: usize, state: SwitchState) {
    // Update this first so that handled events see the latest state in Context
    self.base.switches.update_switch_state(switch_id, state);

    let outgoing_event = match (self.base.switches.by_id(&switch_id), state) {
      (Some(switch), SwitchState::Closed) => {
        log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} closed", switch.name);
        if let Some(hits) = self.base.switches.hits_counter(switch_id) {
          self.base.metrics.inc(hits);
        }
        Some(EventBox::new(SwitchClosed::new(switch.clone())))
      }
      (Some(switch), SwitchState::Open) => {
        log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} opened", switch.name);
        Some(EventBox::new(SwitchOpened::new(switch.clone())))
      }
      (None, _) => {
        // Report as native board/switch id since this is the easiest way to figure out current switch wiring
        match get_native_switch_id(&switch_id, &self.base) {
          Some((board_id, local_id)) => {
            log::warn!(
              target: "frontbox_core::switches",
              "Received event for unknown switch -- board: {}, id: {} -- {:?}",
              board_id,
              local_id,
              state
            );
            return;
          }
          None => {
            log::warn!(
              target: "frontbox_core::switches",
              "Received event for unknown switch on unknown board {} -- {:?}",
              switch_id,
              state
            );
          }
        }
        None
      }
    };

    if let Some(event) = outgoing_event {
      self.emit_event(sender, event);
      self.tracers.send(TraceEvent::SwitchStateChange { switch_id, state });
    }
  }

  fn spawn_system(&mut self, system: SystemContainer, group: GroupId) {
    let system_id = system.id();

    let was_spawned = match self.groups.get_mut(&group) {
      Some(parent) => match parent.id_by_type_id(&system.type_id()) {
        Some(existing_id) => {
          log::warn!(
            "Could not spawn system {} because group {} already has one ({}). Only one system of a type may run per group.",
            system.name(),
            group,
            existing_id
          );
          false
        }
        None => {
          parent.insert(system);
          true
        }
      },
      None => {
        log::warn!(
          "Could not spawn system {} because group {} does not exist.",
          system.name(),
          group
        );
        false
      }
    };

    if was_spawned {
      let handle = SystemHandle::new(system_id, group);
      let mut system = self.groups.system(handle).unwrap();
      log::info!(
        target: "frontbox_core::systems::lifecycle",
        "🌐 Spawned system {} ({})", system.name(), system.id());

      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      guard_system(&mut system, handle, &self.app_sender, "on_spawn", |system| system.initialize(&ctx));

      self.tracers.send(TraceEvent::SystemSpawned {
        id: system_id,
        name: system.name(),
        group,
      });

      let event = SystemSpawned::new(system_id, group);
      let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));
    }
  }

  fn replace_system(&mut self, old_handle: SystemHandle, new_system: SystemContainer) -> Vec<SystemHandle> {
    let despawned = self.despawn_system(old_handle);
    self.spawn_system(new_system, old_handle.group);
    despawned
  }

  /// Despawns the system, along with the groups it owns. Returns the handles of every system despawned: the system
  /// itself, then those in its groups.
  fn despawn_system(&mut self, handle: SystemHandle) -> Vec<SystemHandle> {
    let Some(cell) = self
      .groups
      .get_mut(&handle.group)
      .and_then(|parent| parent.remove(handle.id))
    else {
      return Vec::new();
    };

    {
      let mut system = cell.borrow_mut();
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      // already removed, so a panic here only needs logging
      call_system(&mut system, "on_despawn", |system| system.on_despawn(&ctx));

      log::info!(
        target: "frontbox_core::systems::lifecycle",
        "🌐 Despawned system {} ({})", system.name(), system.id());

      self.tracers.send(TraceEvent::SystemDespawned {
        id: handle.id,
        group: handle.group,
      });

      let event = SystemDespawned::new(handle.id, handle.group);
      let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));
    }

    for state in clear_states_owned_by(&mut self.groups, handle) {
      self.tracers.send(TraceEvent::StateChange {
        group: handle.group,
        name: state.name,
        value: None,
      });
      if let Some(changed) = state.changed_to_default() {
        let _ = self.app_sender.send(EmitEvent(Some(handle.id), changed));
      }
    }

    let mut despawned = vec![handle];
    let owned: Vec<GroupId> = self
      .groups
      .iter()
      .filter(|(_, group)| group.owner == Some(handle))
      .map(|(id, _)| *id)
      .collect();
    for group in owned {
      despawned.extend(self.despawn_system_group(group));
    }
    despawned
  }

  fn spawn_system_group(
    &mut self,
    owner: SystemHandle,
    group: GroupHandle,
    child_systems: Vec<ChildSystemContainer>,
    active: bool,
  ) {
    let id = group.id();
    self.groups.insert(id, SystemGroup::owned_by(group.name(), owner));
    log::info!("🌐 Spawned system group {} ({})", group.name(), id);

    self.tracers.send(TraceEvent::SystemGroupSpawned {
      id,
      name: group.name(),
    });

    for child in child_systems {
      self.spawn_system(child.to_system_container(), id);
    }

    if !active {
      self.deactivate_system_group(id);
    }
  }

  /// Despawns the group and its systems, returning the handles of every system despawned
  fn despawn_system_group(&mut self, id: GroupId) -> Vec<SystemHandle> {
    let Some(name) = self.groups.get(&id).map(|group| group.name) else {
      log::warn!("No system group {} found, cannot despawn", id);
      return Vec::new();
    };

    let mut despawned = Vec::new();
    for handle in self.groups.handles_in(id) {
      despawned.extend(self.despawn_system(handle));
    }

    self.tracers.send(TraceEvent::SystemGroupDespawned { id, name });

    let _ = self.groups.shift_remove(&id);
    log::info!("🌐 Despawned system group {} ({})", name, id);
    despawned
  }

  fn activate_system_group(&mut self, id: GroupId) {
    let Some(group) = self.groups.get_mut(&id) else {
      log::warn!("No system group {} found, cannot activate", id);
      return;
    };

    if !group.active {
      group.active = true;
    } else {
      return;
    }

    for handle in self.groups.handles_in(id) {
      let mut system = self.groups.system(handle).unwrap();
      if system.is_faulted() {
        continue;
      }
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);

      let reactivated = guard_system(&mut system, handle, &self.app_sender, "on_reactivate", |system| {
        let active = system.is_active(&ctx);
        if active {
          system.on_reactivate(&ctx);
        }
        active
      });
      if reactivated == Some(true) {
        emit_activity_changed(handle, true, &self.app_sender);
      }
    }

    self.tracers.send(TraceEvent::SystemGroupActiveStateChange { id, active: true });
  }

  fn deactivate_system_group(&mut self, id: GroupId) {
    let Some(group) = self.groups.get_mut(&id) else {
      log::warn!("No system group {} found, cannot deactivate", id);
      return;
    };

    if group.active {
      group.active = false;
    } else {
      return;
    }

    for handle in self.groups.handles_in(id) {
      let mut system = self.groups.system(handle).unwrap();
      if system.is_faulted() {
        continue;
      }
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      if guard_system(&mut system, handle, &self.app_sender, "on_deactivate", |system| system.on_deactivate(&ctx))
        .is_some()
      {
        emit_activity_changed(handle, false, &self.app_sender);
      }
    }

    self.tracers.send(TraceEvent::SystemGroupActiveStateChange { id, active: false });
  }

  fn create_cue(&self, handle: SystemHandle, cue_id: u64, cue: Cue, signals: Vec<Box<dyn Event>>) {
    if let Some(mut system) = self.groups.system(handle) {
      log::debug!("Creating cue for {} ({})", system.name(), handle.id);
      system.create_cue(cue, cue_id, signals);
    } else {
      log::warn!(
        "Cueing: No system found with ID {}, cannot create cue {:?}",
        handle.id,
        cue
      );
    }
  }

  fn cancel_cue(&self, handle: SystemHandle, cue_id: u64) {
    if let Some(mut system) = self.groups.system(handle) {
      system.cancel_cue(cue_id);
    } else {
      log::warn!(
        "No system found with ID {}, cannot cancel cue with ID {}",
        handle.id,
        cue_id
      );
    }
  }
}

/// Find and apply the closure to the system. Returns None if not found or inactive.
pub(crate) fn apply_to_system<F, T>(
  handle: SystemHandle,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  mut handler: F,
) -> Option<T>
where
  F: FnMut(&mut SystemContainer, &mut SystemContext) -> T,
{
  let mut result: Option<T> = None;

  if groups.get(&handle.group).is_some_and(|group| group.active)
    && let Some(mut system) = groups.system(handle)
    && !system.is_faulted()
  {
    let mut ctx = SystemContext::new(base, handle, groups, &app_sender);
    result = guard_system(&mut system, handle, app_sender, "handler", |system| {
      if handle_active(system, &ctx, app_sender, tracer_txs) {
        Some(handler(system, &mut ctx))
      } else {
        log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
        None
      }
    })
    .flatten();
  }

  result
}

/// Whether the system exists and is active, in an active group. Like any check of activity, this fires the system's
/// deactivate/reactivate handlers if its activity changed since it was last checked.
pub(crate) fn is_system_active(
  handle: SystemHandle,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> bool {
  apply_to_system(handle, groups, base, app_sender, tracer_txs, |_, _| ()).is_some()
}

/// Apply the given closure to all systems, including those within groups, respecting handle_active
pub(crate) fn apply_to_systems<F>(
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &mut SystemContext),
{
  for (key, group) in groups {
    if group.active {
      for cell in group.values() {
        let mut system = cell.borrow_mut();
        if system.is_faulted() {
          continue;
        }
        let handle = SystemHandle::new(system.id(), *key);
        let mut ctx = SystemContext::new(base, handle, groups, app_sender);
        guard_system(&mut system, handle, app_sender, "handler", |system| {
          if handle_active(system, &ctx, app_sender, tracer_txs) {
            handler(system, &mut ctx);
          } else {
            log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
          }
        });
      }
    }
  }
}

/// The first part of `EmitEventOriginFirst`: traces the event, hands it to the origin, and queues the rest of the
/// broadcast behind whatever the origin queued in response
pub(crate) fn emit_event_to_origin(
  origin: SystemHandle,
  event_box: EventBox,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  tracer_txs.send(TraceEvent::Event {
    type_name: event_box.type_name,
    event: event_box.try_json(),
    sender: Some(origin.id),
  });

  apply_to_system(
    origin,
    groups,
    base,
    app_sender,
    tracer_txs,
    |system, ctx| {
      system.on_event(event_box.event.as_ref(), ctx);
    },
  );
  let _ = app_sender.send(AppMessage::EmitEventToOthers(origin, event_box));
}

/// The rest of `EmitEventOriginFirst`: broadcasts to every system but the origin, which already handled the event
pub(crate) fn emit_event_to_others(
  handled_by: SystemHandle,
  event_box: EventBox,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    if ctx.handle() != handled_by {
      system.on_event(event_box.event.as_ref(), ctx);
    }
  });
}

/// Call into the system, catching a panic so it doesn't unwind out of the run loop and end the process. A panic is
/// logged and returns `None`. Use [`guard_system`] unless the system is already on its way out.
///
/// The system may be left half-updated by the panic, which is why `guard_system` retires it. Anything else the
/// handler borrowed is released as the panic unwinds; `RefCell`s aren't poisoned.
fn call_system<T>(
  system: &mut SystemContainer,
  handler: &'static str,
  f: impl FnOnce(&mut SystemContainer) -> T,
) -> Option<T> {
  match catch_unwind(AssertUnwindSafe(|| f(system))) {
    Ok(result) => Some(result),
    Err(payload) => {
      log::error!(
        target: "frontbox_core::systems::lifecycle",
        "💥 System {} ({}) panicked in {}: {}",
        system.name(),
        system.id(),
        handler,
        panic_message(payload.as_ref())
      );
      None
    }
  }
}

/// [`call_system`], and if the system panics, mark it faulted so nothing more is handed to it, and queue its despawn
fn guard_system<T>(
  system: &mut SystemContainer,
  handle: SystemHandle,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  handler: &'static str,
  f: impl FnOnce(&mut SystemContainer) -> T,
) -> Option<T> {
  let result = call_system(system, handler, f);
  if result.is_none() && !system.is_faulted() {
    log::error!(
      target: "frontbox_core::systems::lifecycle",
      "💥 Despawning system {} ({}) after it panicked",
      system.name(),
      system.id()
    );
    system.fault();
    let _ = app_sender.send(AppMessage::DespawnSystem(handle));
  }
  result
}

/// The message a panic was raised with, when it's a string (as `panic!` and `expect` make it)
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> &str {
  payload
    .downcast_ref::<&str>()
    .copied()
    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
    .unwrap_or("<non-string panic payload>")
}

/// Whether the system is active, firing its deactivate/reactivate handler and broadcasting the change if its activity
/// changed since it was last checked
fn handle_active(
  system: &mut SystemContainer,
  ctx: &SystemContext,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> bool {
  let check = system.handle_active(ctx, tracer_txs);
  if check.changed {
    emit_activity_changed(ctx.handle(), check.active, app_sender);
  }
  check.active
}

/// Broadcast [`SystemReactivated`] or [`SystemDeactivated`], once the system's own handler has run
fn emit_activity_changed(
  handle: SystemHandle,
  active: bool,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
) {
  let event = if active {
    EventBox::new(SystemReactivated::new(handle.id, handle.group))
  } else {
    EventBox::new(SystemDeactivated::new(handle.id, handle.group))
  };
  let _ = app_sender.send(EmitEvent(None, event));
}

fn get_native_switch_id(switch_id: &usize, base: &AppState) -> Option<(usize, usize)> {
  let mut offset: usize = 0;
  for (index, board) in base.io_network.iter().enumerate() {
    if *switch_id < (board.switch_count as usize) + offset {
      let native_switch_id = switch_id - offset;
      return Some((index, native_switch_id));
    }
    offset += board.switch_count as usize;
  }
  None
}

/// The most ticks' worth of time a single tick may pass to systems. A longer stall still advances game time, but by
/// at most this many intervals, so systems don't see one enormous step.
const MAX_TICK_INTERVALS: u32 = 4;

/// The `delta` passed to systems for a tick: the measured time since the previous tick, clamped to
/// `MAX_TICK_INTERVALS` intervals.
fn tick_delta(elapsed: Duration, interval: Duration) -> Duration {
  elapsed.min(interval * MAX_TICK_INTERVALS)
}

fn spawn_system_ticker(tick: Duration, sender: watch::Sender<()>, resync: Arc<Notify>) {
  let mut timer_interval = tokio::time::interval(tick);
  timer_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

  tokio::spawn(async move {
    loop {
      tokio::select! {
          _ = timer_interval.tick() => {
              let _ = sender.send(());
          }
          _ = resync.notified() => {
              timer_interval.reset_immediately();
          }
      }
    }
  });
}

/// `Synchronize` is a special event recognized by Frontbox to re-start the internal loop in cadence with receiving this event.
#[derive(serde::Serialize, Event, PartialEq, Eq)]
pub struct Synchronize;

static SYNCHRONIZE_TYPE_ID: TypeId = TypeId::of::<Synchronize>();

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Clone)]
  struct Owner;

  impl System for Owner {}

  #[derive(Clone)]
  struct Member;

  impl System for Member {}

  struct Harness {
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
    fn new() -> Self {
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
    fn activity_events(&mut self) -> Vec<(u64, bool)> {
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

    fn spawn_owner(&mut self) -> SystemHandle {
      self.spawn(Owner)
    }

    fn spawn_group(&mut self, owner: SystemHandle, name: &'static str) -> GroupId {
      let group = GroupHandle::new(GroupId::next(), name);
      let id = group.id();
      self.spawn_system_group(owner, group, vec![Member.into()], true);
      id
    }

    fn despawn(&mut self, handle: SystemHandle) -> Vec<SystemHandle> {
      self.despawn_system(handle)
    }
  }

  #[test]
  fn despawning_an_owner_despawns_its_groups() {
    let mut h = Harness::new();
    let owner = h.spawn_owner();
    let other = h.spawn_owner();
    let player1 = h.spawn_group(owner, "player1");
    let player2 = h.spawn_group(owner, "player2");
    let others = h.spawn_group(other, "other");
    let members: Vec<SystemHandle> = [player1, player2]
      .into_iter()
      .flat_map(|group| h.groups.handles_in(group))
      .collect();

    let despawned = h.despawn(owner);

    assert!(!h.groups.contains_key(&player1));
    assert!(!h.groups.contains_key(&player2));
    assert!(h.groups.contains_key(&others));
    assert_eq!(despawned, [vec![owner], members].concat());
  }

  #[test]
  fn groups_with_the_same_name_are_separate() {
    let mut h = Harness::new();
    let first_owner = h.spawn_owner();
    let second_owner = h.spawn_owner();
    let first = h.spawn_group(first_owner, "player1");
    let second = h.spawn_group(second_owner, "player1");

    h.despawn(first_owner);

    assert!(!h.groups.contains_key(&first));
    assert_eq!(h.groups[&second].owner, Some(second_owner));
    assert_eq!(h.groups.handles_in(second).len(), 1);
  }

  /// Active while its flag is set
  struct Toggled(std::rc::Rc<std::cell::Cell<bool>>);

  impl System for Toggled {
    fn is_active(&self, _ctx: &SystemContext) -> bool {
      self.0.get()
    }
  }

  #[test]
  fn a_system_changing_its_own_activity_is_broadcast() {
    let mut h = Harness::new();
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let id = h.spawn(Toggled(active.clone())).id;
    h.activity_events();
    let check = |h: &Harness| apply_to_systems(&h.groups, &h.base, &h.app_sender, &h.tracers, |_, _| ());

    check(&h);
    assert_eq!(h.activity_events(), vec![]);

    active.set(false);
    check(&h);
    check(&h);
    assert_eq!(h.activity_events(), vec![(id, false)]);

    active.set(true);
    check(&h);
    assert_eq!(h.activity_events(), vec![(id, true)]);
  }

  #[test]
  fn a_group_changing_activity_is_broadcast_for_its_systems() {
    let mut h = Harness::new();
    let owner = h.spawn_owner();
    let group = h.spawn_group(owner, "group");
    let members: Vec<(u64, bool)> = h
      .groups
      .handles_in(group)
      .iter()
      .map(|m| (m.id, false))
      .collect();
    h.activity_events();

    h.deactivate_system_group(group);
    assert_eq!(h.activity_events(), members);

    h.activate_system_group(group);
    let reactivated: Vec<(u64, bool)> = members.iter().map(|(id, _)| (*id, true)).collect();
    assert_eq!(h.activity_events(), reactivated);
  }

  #[derive(serde::Serialize, Event)]
  struct Poke;

  /// Records, on each `Poke`, whether it could look itself up
  struct SelfLookup(std::rc::Rc<std::cell::RefCell<Vec<bool>>>);

  impl System for SelfLookup {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if event.is::<Poke>() {
        let found = ctx.systems.get::<SelfLookup>().is_some();
        self.0.borrow_mut().push(found);
      }
    }
  }

  /// Calls into `Callee`, which looks this system up again
  struct Caller;

  impl System for Caller {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if event.is::<Poke>() {
        ctx.systems.with::<Callee, _>(|callee| callee.call_back(ctx.into()));
      }
    }
  }

  struct Callee {
    handle: SystemHandle,
    found_caller: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
  }

  impl Callee {
    fn call_back(&mut self, ctx: &ServiceContext) {
      let ctx = ctx.for_system(self.handle);
      let found = ctx.systems.get::<Caller>().is_some();
      self.found_caller.borrow_mut().push(found);
    }
  }

  impl System for Callee {
    fn on_spawn(&mut self, ctx: &SystemContext) {
      self.handle = ctx.handle();
    }
  }

  /// Panics on `Poke`
  struct Panicky;

  impl System for Panicky {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if event.is::<Poke>() {
        panic!("poked");
      }
    }
  }

  /// Panics on its way out
  struct PanicsOnDespawn;

  impl System for PanicsOnDespawn {
    fn on_despawn(&mut self, _ctx: &SystemContext) {
      panic!("despawned");
    }
  }

  /// Counts the `Poke`s it hears
  struct Counter(std::rc::Rc<std::cell::Cell<u32>>);

  impl System for Counter {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if event.is::<Poke>() {
        self.0.set(self.0.get() + 1);
      }
    }
  }

  impl Harness {
    fn spawn(&mut self, system: impl Into<SystemContainer>) -> SystemHandle {
      let container = system.into();
      let handle = SystemHandle::new(container.id(), ROOT_GROUP);
      self.spawn_system(container, ROOT_GROUP);
      handle
    }

    fn poke(&self) {
      self.emit_event(None, EventBox::new(Poke));
    }

    /// The handles of the `DespawnSystem` messages sent so far
    fn despawns_requested(&mut self) -> Vec<SystemHandle> {
      let mut handles = Vec::new();
      while let Ok(message) = self.rx.try_recv() {
        if let AppMessage::DespawnSystem(handle) = message {
          handles.push(handle);
        }
      }
      handles
    }
  }

  #[test]
  fn a_system_looking_itself_up_gets_none() {
    let mut h = Harness::new();
    let found = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    h.spawn(SelfLookup(found.clone()));

    h.poke();

    assert_eq!(*found.borrow(), vec![false]);
  }

  #[test]
  fn a_lookup_that_closes_a_cycle_gets_none() {
    let mut h = Harness::new();
    let found = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    h.spawn(Caller);
    h.spawn(Callee {
      handle: SystemHandle::default(),
      found_caller: found.clone(),
    });

    h.poke();

    assert_eq!(*found.borrow(), vec![false]);
  }

  #[test]
  fn a_panicking_system_is_retired_and_the_rest_carry_on() {
    let mut h = Harness::new();
    let heard = std::rc::Rc::new(std::cell::Cell::new(0));
    let before = h.spawn(Counter(heard.clone()));
    let panicky = h.spawn(Panicky);
    h.despawns_requested();

    h.poke();
    h.poke();

    // systems after it still hear the event, and it's only despawned once
    assert_eq!(heard.get(), 2);
    assert_eq!(h.despawns_requested(), vec![panicky]);
    assert!(h.groups.system(panicky).unwrap().is_faulted());
    assert!(!h.groups.system(before).unwrap().is_faulted());
  }

  #[test]
  fn a_panic_while_despawning_still_despawns() {
    let mut h = Harness::new();
    let handle = h.spawn(PanicsOnDespawn);

    assert_eq!(h.despawn(handle), vec![handle]);
    assert!(!h.groups.contains_system(handle));
  }

  #[test]
  fn tick_delta_is_the_measured_time_clamped_to_a_few_intervals() {
    let interval = Duration::from_millis(80);
    assert_eq!(tick_delta(Duration::from_millis(95), interval), Duration::from_millis(95));
    assert_eq!(tick_delta(Duration::from_millis(40), interval), Duration::from_millis(40));
    assert_eq!(tick_delta(Duration::from_secs(3), interval), interval * MAX_TICK_INTERVALS);
  }
}
