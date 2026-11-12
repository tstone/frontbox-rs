use fast_protocol::SwitchState;
use std::any::TypeId;
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

pub async fn run(
  mut base: AppState,
  initial_systems: Vec<SystemContainer>,
  app_tracers: Vec<Box<dyn AppTracer>>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  mut app_receiver: mpsc::UnboundedReceiver<AppMessage>,
  mut tracer_ctl_receiver: mpsc::UnboundedReceiver<TracerControlEvent>,
  machine_system_id: u64,
  mut routines: Routines,
) {
  let (tick_tx, mut tick_rx) = watch::channel(());
  let tracer_txs = TracerSenders::new(app_tracers.iter().map(|tracer| tracer.sender()).collect());
  let resync_notifier = Arc::new(Notify::new());

  let mut groups: Groups = Groups::new();

  groups.insert(ROOT_GROUP, SystemGroup::new(ROOT_GROUP_NAME));
  tracer_txs.send(TraceEvent::SystemGroupSpawned {
    id: ROOT_GROUP,
    name: ROOT_GROUP_NAME,
  });

  // initialize root systems
  for system in initial_systems {
    spawn_system(
      system,
      ROOT_GROUP,
      &mut groups,
      &base,
      app_sender.clone(),
      &tracer_txs,
    );
  }
  spawn_system_ticker(base.system_interval, tick_tx, resync_notifier.clone());
  trace_configs(&base, &tracer_txs);

  // listen for ctrl-c to trigger shutdown
  let tx = app_sender.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to listen for ctrl-c");
    log::info!("Ctrl+C signal received.");
    let _ = tx.send(AppMessage::Shutdown(ShutdownScope::Process));
  });

  let mut metrics_flush = tokio::time::interval_at(
    tokio::time::Instant::now() + metrics::FLUSH_INTERVAL,
    metrics::FLUSH_INTERVAL,
  );

  log::info!(target: "frontbox_core::run_loop", "⟳ Run loop started.");
  let shutdown_scope;
  let mut last_tick = Instant::now();

  loop {
    tokio::select! {
      Some(command) = app_receiver.recv() => {
        log::trace!(target: "frontbox_core::run_loop", "Run loop: AppMessage queue depth: {}", app_receiver.len());
        log::trace!(target: "frontbox_core::run_loop", "Run loop: Now handling {:?}", command);
        let cmd_text = format!("{:?}", command);
        let start = std::time::Instant::now();

        match command {
          AppMessage::EmitEvent(sender, event_box) => {
            emit_event(sender, event_box, &mut groups, &base, &app_sender, &tracer_txs, &resync_notifier);
          }
          AppMessage::EmitEventOriginFirst(origin, event_box) => {
            emit_event_to_origin(origin, event_box, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::EmitEventToOthers(handled_by, event_box) => {
            emit_event_to_others(handled_by, event_box, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::SwitchStateChange(id, state) => {
            switch_state_changed(Some(machine_system_id), id, state, &mut groups, &mut base, &app_sender, &tracer_txs, &resync_notifier);
          }
          AppMessage::SyncSwitchStates(switch_states) => {
            base.switches.update_switch_states(switch_states);
          }
          AppMessage::ConfigChanged(key) => {
            base.config.save();
            emit_event(None, EventBox::new(ConfigChanged(key)), &mut groups, &base, &app_sender, &tracer_txs, &resync_notifier);
          }
          AppMessage::DriverModeChange(driver_id, mode) => {
            base.drivers.set_mode(driver_id, mode.clone());
            tracer_txs.send(TraceEvent::DriverModeChange { driver_id, mode });
          }
          AppMessage::Shutdown(scope) => {
            log::warn!(target: "frontbox_core::run_loop", "⏹️ Shutdown command received, shutting down...");
            shutdown_scope = scope;
            break;
          }
          AppMessage::SpawnSystem(group, system) => {
            spawn_system(system.to_system_container(), group, &mut groups, &base, app_sender.clone(), &tracer_txs);
          }
          // Routines hear about despawns and group activity here, after the change, rather than through a message
          // of their own, so they see it in queue order: a group deactivated and then reactivated still ends a step.
          AppMessage::ReplaceSystem(handle, system) => {
            let despawned = replace_system(handle, system.to_system_container(), &mut groups, &base, app_sender.clone(), &tracer_txs);
            routines.system_activation_changed(&despawned, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::DespawnSystem(handle) => {
            let despawned = despawn_system(handle, &mut groups, &base, app_sender.clone(), &tracer_txs);
            routines.system_activation_changed(&despawned, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::SpawnSystemGroup(owner, group, child_systems, active) => {
            spawn_system_group(owner, group, child_systems, active, &mut groups, &base, app_sender.clone(), &tracer_txs);
          }
          AppMessage::DespawnSystemGroup(group) => {
            let despawned = despawn_system_group(group, &mut groups, &base, app_sender.clone(), &tracer_txs);
            routines.system_activation_changed(&despawned, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::ActivateSystemGroup(group) => {
            activate_system_group(group, &mut groups, &base, app_sender.clone(), &tracer_txs);
            routines.system_activation_changed(&groups.handles_in(group), &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::DeactivateSystemGroup(group) => {
            deactivate_system_group(group, &mut groups, &base, app_sender.clone(), &tracer_txs);
            routines.system_activation_changed(&groups.handles_in(group), &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::CreateCue(handle, cue_id, cue, signals) => {
            create_cue(handle, cue_id, cue, signals, &groups);
          }
          AppMessage::CreateCueTimeline(handle, cue_id, timeline) => {
            create_cue_timeline(handle, cue_id, timeline, &groups);
          }
          AppMessage::CancelCue(handle, cue_id) => {
            cancel_cue(handle, cue_id, &groups);
          }
          AppMessage::StartRoutine(start) => {
            let (origin, type_id) = (start.origin, start.type_id);
            if let Some(started) = routines.start(start, &groups, &tracer_txs) {
              // emitted here and the first step queued after it, so whatever its listeners emit lands first
              emit_event(Some(origin.id), started, &mut groups, &base, &app_sender, &tracer_txs, &resync_notifier);
              let _ = app_sender.send(AppMessage::BeginRoutine(type_id));
            }
          }
          AppMessage::BeginRoutine(type_id) => {
            routines.begin(type_id, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::CompleteRoutineStep(handle, type_id) => {
            routines.complete_step(handle, type_id, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::HaltRoutine(handle, type_id) => {
            routines.halt(handle, type_id, &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::SystemActivityChanged(handle) => {
            routines.system_activation_changed(&[handle], &groups, &base, &app_sender, &tracer_txs);
          }
          AppMessage::SetState(state) => {
            let (owner, name, value) = (state.owner, state.name, (state.to_json)(state.value.as_ref()));
            if let Some(changed) = set_state(&mut groups, state) {
              tracer_txs.send(TraceEvent::StateChange {
                group: owner.group,
                name,
                value: Some(value),
              });
              emit_event(Some(owner.id), changed, &mut groups, &base, &app_sender, &tracer_txs, &resync_notifier);
            }
          }
          AppMessage::TracerEvent(event) => {
            tracer_txs.send(event);
          }
        }

        let duration = start.elapsed().as_micros();
        if duration > 1000 {
          log::warn!(target: "frontbox_core::run_loop", "Run loop Took {}μ to process {}", duration, cmd_text);
        } else {
          log::trace!(target: "frontbox_core::run_loop", "Run loop: Took {}μ", duration);
        }
      }

      Ok(_) = tick_rx.changed() => {
        let now = Instant::now();
        let delta = tick_delta(now - last_tick, base.system_interval);
        last_tick = now;
        handle_system_tick(delta, &mut groups, &base, &app_sender, &tracer_txs).await;
      }

      _ = metrics_flush.tick() => {
        base.metrics.flush();
      }

      Some(tracer_command) = tracer_ctl_receiver.recv() => {
        match tracer_command {
          TracerControlEvent::OpenSwitch { switch_id } => {
            if base.switches.is_closed_by_id(switch_id).unwrap_or(false) {
              log::warn!(target: "frontbox_core::run_loop", "Tracer forcibly opening switch {switch_id}...");
              switch_state_changed(None, switch_id, SwitchState::Open, &mut groups, &mut base, &app_sender, &tracer_txs, &resync_notifier);
            }
          }
          TracerControlEvent::CloseSwitch { switch_id } => {
            if base.switches.is_open_by_id(switch_id).unwrap_or(false) {
              log::warn!(target: "frontbox_core::run_loop", "Tracer forcibly closing switch {switch_id}...");
              switch_state_changed(None, switch_id, SwitchState::Closed, &mut groups, &mut base, &app_sender, &tracer_txs, &resync_notifier);
            }
          }
          TracerControlEvent::SetConfig { key, value } => {
            // saved, emitted and applied to hardware through `AppMessage::ConfigChanged`, like a menu edit
            match base.config.set_by_key(&key, value) {
              Ok(()) => log::info!(target: "frontbox_core::run_loop", "Tracer set config '{key}' to {value:?}"),
              Err(err) => log::warn!(target: "frontbox_core::run_loop", "Tracer couldn't set config: {err}"),
            }
          }
        }
      }
    }
  }

  // Shutdown sequence
  apply_to_systems(
    &mut groups,
    &base,
    &app_sender,
    &tracer_txs,
    |system, ctx| {
      system.on_despawn(ctx);
    },
  );

  // after despawning, so anything systems record on their way out is kept
  base.metrics.close().await;

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1500)).await;

  if shutdown_scope == ShutdownScope::OperatingSystem {
    if let Err(e) = Command::new("sudo").args(["shutdown"]).status() {
      log::error!(target: "frontbox_core::run_loop", "Failed to invoke OS shutdown: {e}");
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
  {
    let mut ctx = SystemContext::new(base, handle, groups, &app_sender);
    if handle_active(&mut system, &ctx, app_sender, tracer_txs) {
      result = Some(handler(&mut system, &mut ctx));
    } else {
      log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
    }
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
        let mut ctx = SystemContext::new(
          base,
          SystemHandle::new(system.id(), *key),
          groups,
          app_sender,
        );
        if handle_active(&mut system, &ctx, app_sender, tracer_txs) {
          handler(&mut system, &mut ctx);
        } else {
          log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
        }
      }
    }
  }
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

fn emit_event(
  sender: Option<u64>,
  event_box: EventBox,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  resync_notifier: &Arc<Notify>,
) {
  if event_box.type_id == SYNCHRONIZE_TYPE_ID {
    resync_notifier.notify_one();
  }

  // notify any monitoring tracers
  tracer_txs.send(TraceEvent::Event {
    type_name: event_box.type_name,
    event: event_box.try_json(),
    sender,
  });

  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    system.on_event(event_box.event.as_ref(), ctx);
  });
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

/// Tells tracers every config in use. Hardware configs are all known at boot, so they're listed even if nothing has
/// read them yet; software configs join as they're first used.
fn trace_configs(base: &AppState, tracer_txs: &TracerSenders) {
  for (_, mode) in base.drivers.modes() {
    for config in mode.configs() {
      base.config.inspect(config);
    }
  }
  tracer_txs.send(TraceEvent::ConfigsInUse {
    configs: base.config.views(),
  });
}

fn switch_state_changed(
  sender: Option<u64>,
  switch_id: usize,
  state: SwitchState,
  groups: &Groups,
  base: &mut AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  resync_notifier: &Arc<Notify>,
) {
  // Update this first so that handled events see the latest state in Context
  base.switches.update_switch_state(switch_id, state);

  let outgoing_event = match (base.switches.by_id(&switch_id), state) {
    (Some(switch), SwitchState::Closed) => {
      log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} closed", switch.name);
      if let Some(hits) = base.switches.hits_counter(switch_id) {
        base.metrics.inc(hits);
      }
      Some(EventBox::new(SwitchClosed::new(switch.clone())))
    }
    (Some(switch), SwitchState::Open) => {
      log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} opened", switch.name);
      Some(EventBox::new(SwitchOpened::new(switch.clone())))
    }
    (None, _) => {
      // Report as native board/switch id since this is the easiest way to figure out current switch wiring
      match get_native_switch_id(&switch_id, base) {
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
    emit_event(
      sender,
      event,
      groups,
      base,
      app_sender,
      tracer_txs,
      resync_notifier,
    );
    tracer_txs.send(TraceEvent::SwitchStateChange { switch_id, state });
  }
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

async fn handle_system_tick(
  delta: Duration,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    // Tick first is where most systems should do any time-based processing
    system.on_tick(delta, ctx);
    // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
    system.on_render(ctx);
  });
}

fn spawn_system(
  system: SystemContainer,
  group: GroupId,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let system_id = system.id();

  let was_spawned = if let Some(parent) = groups.get_mut(&group) {
    parent.insert(system);
    true
  } else {
    log::warn!(
      "Could not spawn system {} because group {} does not exist.",
      system.name(),
      group
    );
    false
  };

  if was_spawned {
    let handle = SystemHandle::new(system_id, group);
    let mut system = groups.system(handle).unwrap();
    log::info!(
      target: "frontbox_core::systems::lifecycle",
      "🌐 Spawned system {} ({})", system.name(), system.id());

    let ctx = SystemContext::new(base, handle, groups, &app_sender);
    system.initialize(&ctx);

    tracer_txs.send(TraceEvent::SystemSpawned {
      id: system_id,
      name: system.name(),
      group,
    });

    let event = SystemSpawned::new(system_id, group);
    let _ = app_sender.send(EmitEvent(None, EventBox::new(event)));
  }
}

fn replace_system(
  old_handle: SystemHandle,
  new_system: SystemContainer,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> Vec<SystemHandle> {
  let despawned = despawn_system(old_handle, groups, base, app_sender.clone(), tracer_txs);
  spawn_system(
    new_system,
    old_handle.group,
    groups,
    base,
    app_sender,
    tracer_txs,
  );
  despawned
}

/// Despawns the system, along with the groups it owns. Returns the handles of every system despawned: the system
/// itself, then those in its groups.
fn despawn_system(
  handle: SystemHandle,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> Vec<SystemHandle> {
  let Some(cell) = groups
    .get_mut(&handle.group)
    .and_then(|parent| parent.remove(handle.id))
  else {
    return Vec::new();
  };

  {
    let mut system = cell.borrow_mut();
    let ctx = SystemContext::new(base, handle, groups, &app_sender);
    system.on_despawn(&ctx);

    log::info!(
      target: "frontbox_core::systems::lifecycle",
      "🌐 Despawned system {} ({})", system.name(), system.id());

    tracer_txs.send(TraceEvent::SystemDespawned {
      id: handle.id,
      group: handle.group,
    });

    let event = SystemDespawned::new(handle.id, handle.group);
    let _ = app_sender.send(EmitEvent(None, EventBox::new(event)));
  }

  for state in clear_states_owned_by(groups, handle) {
    tracer_txs.send(TraceEvent::StateChange {
      group: handle.group,
      name: state.name,
      value: None,
    });
    if let Some(changed) = state.changed_to_default() {
      let _ = app_sender.send(EmitEvent(Some(handle.id), changed));
    }
  }

  let mut despawned = vec![handle];
  let owned: Vec<GroupId> = groups
    .iter()
    .filter(|(_, group)| group.owner == Some(handle))
    .map(|(id, _)| *id)
    .collect();
  for group in owned {
    despawned.extend(despawn_system_group(
      group,
      groups,
      base,
      app_sender.clone(),
      tracer_txs,
    ));
  }
  despawned
}

fn spawn_system_group(
  owner: SystemHandle,
  group: GroupHandle,
  child_systems: Vec<ChildSystemContainer>,
  active: bool,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let id = group.id();
  groups.insert(id, SystemGroup::owned_by(group.name(), owner));
  log::info!("🌐 Spawned system group {} ({})", group.name(), id);

  tracer_txs.send(TraceEvent::SystemGroupSpawned {
    id,
    name: group.name(),
  });

  for child in child_systems {
    spawn_system(
      child.to_system_container(),
      id,
      groups,
      base,
      app_sender.clone(),
      tracer_txs,
    );
  }

  if !active {
    deactivate_system_group(id, groups, base, app_sender, tracer_txs);
  }
}

/// Despawns the group and its systems, returning the handles of every system despawned
fn despawn_system_group(
  id: GroupId,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> Vec<SystemHandle> {
  let Some(name) = groups.get(&id).map(|group| group.name) else {
    log::warn!("No system group {} found, cannot despawn", id);
    return Vec::new();
  };

  let mut despawned = Vec::new();
  for handle in groups.handles_in(id) {
    despawned.extend(despawn_system(
      handle,
      groups,
      base,
      app_sender.clone(),
      tracer_txs,
    ));
  }

  tracer_txs.send(TraceEvent::SystemGroupDespawned { id, name });

  let _ = groups.shift_remove(&id);
  log::info!("🌐 Despawned system group {} ({})", name, id);
  despawned
}

fn activate_system_group(
  id: GroupId,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let Some(group) = groups.get_mut(&id) else {
    log::warn!("No system group {} found, cannot activate", id);
    return;
  };

  if !group.active {
    group.active = true;
  } else {
    return;
  }

  for handle in groups.handles_in(id) {
    let mut system = groups.system(handle).unwrap();
    let ctx = SystemContext::new(base, handle, groups, &app_sender);

    if system.is_active(&ctx) {
      system.on_reactivate(&ctx);
      emit_activity_changed(handle, true, &app_sender);
    }
  }

  tracer_txs.send(TraceEvent::SystemGroupActiveStateChange { id, active: true });
}

fn deactivate_system_group(
  id: GroupId,
  groups: &mut Groups,
  base: &AppState,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let Some(group) = groups.get_mut(&id) else {
    log::warn!("No system group {} found, cannot deactivate", id);
    return;
  };

  if group.active {
    group.active = false;
  } else {
    return;
  }

  for handle in groups.handles_in(id) {
    let mut system = groups.system(handle).unwrap();
    let ctx = SystemContext::new(base, handle, groups, &app_sender);
    system.on_deactivate(&ctx);
    emit_activity_changed(handle, false, &app_sender);
  }

  tracer_txs.send(TraceEvent::SystemGroupActiveStateChange { id, active: false });
}

fn create_cue(
  handle: SystemHandle,
  cue_id: u64,
  cue: Cue,
  signals: Vec<Box<dyn Event>>,
  groups: &Groups,
) {
  if let Some(mut system) = groups.system(handle) {
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

fn create_cue_timeline(handle: SystemHandle, cue_id: u64, timeline: CueTimeline, groups: &Groups) {
  if let Some(mut system) = groups.system(handle) {
    system.create_cue_timeline(timeline, cue_id);
  } else {
    log::warn!(
      "No system found with ID {}, cannot create cue timeline {:?}",
      handle.id,
      cue_id
    );
  }
}

fn cancel_cue(handle: SystemHandle, cue_id: u64, groups: &Groups) {
  if let Some(mut system) = groups.system(handle) {
    system.cancel_cue(cue_id);
  } else {
    log::warn!(
      "No system found with ID {}, cannot cancel cue with ID {}",
      handle.id,
      cue_id
    );
  }
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
    base: AppState,
    groups: Groups,
    tx: mpsc::UnboundedSender<AppMessage>,
    rx: mpsc::UnboundedReceiver<AppMessage>,
    tracers: TracerSenders,
  }

  impl Harness {
    fn new() -> Self {
      let (tx, rx) = mpsc::unbounded_channel();
      let mut groups = Groups::new();
      groups.insert(ROOT_GROUP, SystemGroup::new(ROOT_GROUP_NAME));

      Self {
        base: AppState::default(),
        groups,
        tx,
        rx,
        tracers: TracerSenders::new(Vec::new()),
      }
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
      let container = SystemContainer::new(Owner);
      let handle = SystemHandle::new(container.id(), ROOT_GROUP);
      spawn_system(
        container,
        ROOT_GROUP,
        &mut self.groups,
        &self.base,
        self.tx.clone(),
        &self.tracers,
      );
      handle
    }

    fn spawn_group(&mut self, owner: SystemHandle, name: &'static str) -> GroupId {
      let group = GroupHandle::new(GroupId::next(), name);
      let id = group.id();
      spawn_system_group(
        owner,
        group,
        vec![Member.into()],
        true,
        &mut self.groups,
        &self.base,
        self.tx.clone(),
        &self.tracers,
      );
      id
    }

    fn despawn(&mut self, handle: SystemHandle) -> Vec<SystemHandle> {
      despawn_system(
        handle,
        &mut self.groups,
        &self.base,
        self.tx.clone(),
        &self.tracers,
      )
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
    let container = SystemContainer::new(Toggled(active.clone()));
    let id = container.id();
    spawn_system(
      container,
      ROOT_GROUP,
      &mut h.groups,
      &h.base,
      h.tx.clone(),
      &h.tracers,
    );
    h.activity_events();
    let check = |h: &Harness| apply_to_systems(&h.groups, &h.base, &h.tx, &h.tracers, |_, _| ());

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

    deactivate_system_group(group, &mut h.groups, &h.base, h.tx.clone(), &h.tracers);
    assert_eq!(h.activity_events(), members);

    activate_system_group(group, &mut h.groups, &h.base, h.tx.clone(), &h.tracers);
    let reactivated: Vec<(u64, bool)> = members.iter().map(|(id, _)| (*id, true)).collect();
    assert_eq!(h.activity_events(), reactivated);
  }

  #[test]
  fn tick_delta_is_the_measured_time_clamped_to_a_few_intervals() {
    let interval = Duration::from_millis(80);
    assert_eq!(tick_delta(Duration::from_millis(95), interval), Duration::from_millis(95));
    assert_eq!(tick_delta(Duration::from_millis(40), interval), Duration::from_millis(40));
    assert_eq!(tick_delta(Duration::from_secs(3), interval), interval * MAX_TICK_INTERVALS);
  }
}
