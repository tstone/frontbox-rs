use itertools::Itertools;
use std::any::TypeId;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

use tokio::sync::{Notify, mpsc, watch};
use tokio::time::Duration;

use crate::app::app_message::AppMessage::EmitEvent;
use crate::app::app_tracer::{AppTracer, InterruptEvaluation, TraceEvent};
use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;
use crate::systems::SystemContainer;
use crate::systems::event_interrupts::EventInterruptRegistry;

pub(crate) type TracerSenders = Vec<mpsc::UnboundedSender<app_tracer::TraceEvent>>;

pub async fn run(
  mut base: BootSnapshot,
  initial_systems: Vec<SystemContainer>,
  app_tracers: Vec<Box<dyn AppTracer>>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  mut app_receiver: mpsc::UnboundedReceiver<AppMessage>,
) {
  let (tick_tx, mut tick_rx) = watch::channel(());
  let tracer_txs: TracerSenders = app_tracers.iter().map(|tracer| tracer.sender()).collect();
  let resync_notifier = Arc::new(Notify::new());

  let mut interrupt_registry = EventInterruptRegistry::new();
  let mut groups: Groups = HashMap::new();

  groups.insert(ROOT_GROUP, SystemGroup::new());
  for tracer in &tracer_txs {
    let _ = tracer.send(TraceEvent::SystemGroupSpawned { key: ROOT_GROUP });
  }

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

  // listen for ctrl-c to trigger shutdown
  let tx = app_sender.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to listen for ctrl-c");
    log::info!("Ctrl+C signal received.");
    let _ = tx.send(AppMessage::Shutdown(ShutdownScope::Process));
  });

  log::info!(target: "frontbox::run_loop", "⟳ Run loop started.");
  let mut shutdown_scope = ShutdownScope::Process;

  loop {
    tokio::select! {
      Some(command) = app_receiver.recv() => {
        log::trace!(target: "frontbox::run_loop", "Run loop: AppMessage queue depth: {}", app_receiver.len());
        log::trace!(target: "frontbox::run_loop", "Run loop: Now handling {:?}", command);
        let cmd_text = format!("{:?}", command);
        let start = std::time::Instant::now();

        match command {
          AppMessage::EmitEvent(event_box) => {
            emit_event(event_box, &mut groups, &base, &app_sender, &interrupt_registry, &tracer_txs, &resync_notifier);
          }
          AppMessage::RegisterInterrupt(handle, type_id, priority) => {
            interrupt_registry.register(type_id, handle.id, handle.parent_key, priority);
          }
          AppMessage::UnregisterInterrupt(system_id, type_id) => {
            interrupt_registry.unregister(system_id, type_id);
          }
          AppMessage::UnregisterAllBySystem(system_id) => {
            unregister_all_by_system(&system_id, &mut interrupt_registry);
          }
          AppMessage::SingleSwitchState(id, state) => {
            base.switches.update_switch_state(id, state);
          }
          AppMessage::SwitchStates(switch_states) => {
            base.switches.update_switch_states(switch_states);
          }
          AppMessage::Shutdown(scope) => {
            log::warn!(target: "frontbox::run_loop", "⏹️ Shutdown command received, shutting down...");
            shutdown_scope = scope;
            break;
          }
          AppMessage::SpawnSystem(parent_key, system) => {
            spawn_system(system.to_system_container(), parent_key, &mut groups, &base, app_sender.clone(), &tracer_txs);
          }
          AppMessage::ReplaceSystem(handle, system) => {
            replace_system(handle, system.to_system_container(), &mut groups, &base, app_sender.clone(), &mut interrupt_registry, &tracer_txs);
          }
          AppMessage::DespawnSystem(handle) => {
            despawn_system(handle, &mut groups, &base, app_sender.clone(), &mut interrupt_registry, &tracer_txs);
          }
          AppMessage::SpawnSystemGroup(group_name, child_systems, active) => {
            spawn_system_group(group_name, child_systems, active, &mut groups, &base, app_sender.clone(), &tracer_txs);
          }
          AppMessage::DespawnSystemGroup(group_name) => {
            despawn_system_group(group_name, &mut groups, &base, app_sender.clone(), &mut interrupt_registry, &tracer_txs);
          }
          AppMessage::ActivateSystemGroup(group_name) => {
            activate_system_group(group_name, &mut groups, &base, app_sender.clone(), &tracer_txs);
          }
          AppMessage::DeactivateSystemGroup(group_name) => {
            deactivate_system_group(group_name, &mut groups, &base, app_sender.clone(), &tracer_txs);
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
        }

        let duration = start.elapsed().as_micros();
        if duration > 1000 {
          log::warn!(target: "frontbox::run_loop", "Run loop Took {}μ to process {}", duration, cmd_text);
        } else {
          log::trace!(target: "frontbox::run_loop", "Run loop: Took {}μ", duration);
        }
      }

      Ok(_) = tick_rx.changed() => {
        handle_system_tick(&mut groups, &base, &app_sender, &tracer_txs).await;
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

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1500)).await;

  if shutdown_scope == ShutdownScope::OperatingSystem {
    if let Err(e) = Command::new("sudo").args(["shutdown"]).status() {
      log::error!(target: "frontbox::run_loop", "Failed to invoke OS shutdown: {e}");
    }
  }
}

/// Find and apply the closure to the system. Returns None if not found or inactive.
fn apply_to_system<F, T>(
  handle: SystemHandle,
  groups: &Groups,
  base: &BootSnapshot,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  mut handler: F,
) -> Option<T>
where
  F: FnMut(&mut SystemContainer, &mut SystemContext) -> T,
{
  let mut result: Option<T> = None;

  if let Some(group) = groups.get(handle.parent_key)
    && group.active
    && let Some(mut system) = group.get_by_id(&handle.id)
  {
    let mut ctx = SystemContext::new(base, handle, groups, app_sender.clone());
    if system.handle_active(&ctx, tracer_txs) {
      result = Some(handler(&mut system, &mut ctx));
    } else {
      log::trace!(target: "frontbox::inactive", "System {} is inactive, skipping", system.id());
    }
  }

  result
}

/// Apply the given closure to all systems, including those within groups, respecting handle_active
fn apply_to_systems<F>(
  groups: &Groups,
  base: &BootSnapshot,
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
          SystemHandle::new(system.id(), key),
          groups,
          app_sender.clone(),
        );
        if system.handle_active(&ctx, tracer_txs) {
          handler(&mut system, &mut ctx);
        } else {
          log::trace!(target: "frontbox::inactive", "System {} is inactive, skipping", system.id());
        }
      }
    }
  }
}

fn emit_event(
  event_box: EventBox,
  groups: &Groups,
  base: &BootSnapshot,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &EventInterruptRegistry,
  tracer_txs: &TracerSenders,
  resync_notifier: &Arc<Notify>,
) {
  if event_box.type_id == SYNCHRONIZE_TYPE_ID {
    resync_notifier.notify_one();
  }

  let mut interrupt_evals = Vec::new();

  // first pass the event through the interrupt registry. If any interrupt returns `Halt`, stop processing further.
  if let Some(interrupts) = interrupt_registry.get_interrupts_for_event(event_box.type_id) {
    // interrupts must be evaluated in order of priority (highest first)
    let prioritized_interrupts = interrupts
      .iter()
      .sorted_by_key(|i| std::cmp::Reverse(i.priority));

    for interrupt in prioritized_interrupts {
      let interrupt_result = apply_to_system(
        interrupt.to_handle(),
        groups,
        base,
        app_sender,
        tracer_txs,
        |system, ctx| system.on_interrupt(event_box.event.as_ref(), &ctx),
      );

      if let Some(result) = interrupt_result {
        interrupt_evals.push(InterruptEvaluation {
          interrupter: interrupt.system_id,
          result,
        });
      }

      if interrupt_result == Some(InterruptResult::Halt) {
        log::info!(
          "Event {} was halted by interrupt in system {}",
          event_box.type_name,
          interrupt.system_id
        );
        return;
      }
    }
  } else {
    log::debug!("No interrupts registered for {}", event_box.type_name);
  }

  // notify any monitoring tracers
  for tracer in tracer_txs {
    let _ = tracer.send(TraceEvent::Event {
      type_name: event_box.type_name,
      interrupts: interrupt_evals.clone(),
      event: event_box.try_json(),
    });
  }

  // event is broadcast to systems if no interrupt halted it
  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    system.on_event(event_box.event.as_ref(), ctx);
  });
}

async fn handle_system_tick(
  groups: &Groups,
  base: &BootSnapshot,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let tick_duration = base.system_interval;

  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    // Tick first is where most systems should do any time-based processing
    system.on_tick(tick_duration, ctx);
    // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
    system.on_render(ctx);
  });
}

fn spawn_system(
  system: SystemContainer,
  parent_key: &'static str,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  let system_id = system.id();

  let was_spawned = if let Some(parent) = groups.get_mut(parent_key) {
    parent.insert(system);
    true
  } else {
    log::warn!(
      "Could not spawn system {} because parent '{}' does not exist.",
      system.name(),
      parent_key
    );
    false
  };

  if was_spawned {
    let mut system = groups
      .get(parent_key)
      .unwrap()
      .systems
      .get_by_id(&system_id)
      .unwrap();
    log::info!(
      target: "frontbox::systems::lifecycle", 
      "🌐 Spawned system {} ({})", system.name(), system.id());

    let ctx = SystemContext::new(
      base,
      SystemHandle::new(system_id, parent_key),
      groups,
      app_sender.clone(),
    );
    system.on_spawn(&ctx);

    for tracer in tracer_txs {
      let _ = tracer.send(TraceEvent::SystemSpawned {
        id: system_id,
        name: system.name(),
        parent_key,
      });
    }

    let event = SystemSpawned::new(system_id, parent_key);
    let _ = app_sender.send(EmitEvent(EventBox::new(event)));
  }
}

fn replace_system(
  old_handle: SystemHandle,
  new_system: SystemContainer,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
  tracer_txs: &TracerSenders,
) {
  despawn_system(
    old_handle,
    groups,
    base,
    app_sender.clone(),
    interrupt_registry,
    tracer_txs,
  );
  spawn_system(
    new_system,
    old_handle.parent_key,
    groups,
    base,
    app_sender,
    tracer_txs,
  );
}

/// Despawns the system, returning true if it succeeded
fn despawn_system(
  handle: SystemHandle,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
  tracer_txs: &TracerSenders,
) {
  if let Some(parent) = groups.get_mut(handle.parent_key)
    && let Some(cell) = parent.remove(handle.id)
  {
    let mut system = cell.borrow_mut();
    let ctx = SystemContext::new(base, handle, groups, app_sender.clone());
    system.on_despawn(&ctx);
    interrupt_registry.unregister_by_system(&handle.id);

    log::info!(
      target: "frontbox::systems::lifecycle", 
      "🌐 Despawned system {} ({})", system.name(), system.id());

    for tracer in tracer_txs {
      let _ = tracer.send(TraceEvent::SystemDespawned {
        id: handle.id,
        parent_key: handle.parent_key,
      });
    }

    let event = SystemDespawned::new(handle.id, handle.parent_key);
    let _ = app_sender.send(EmitEvent(EventBox::new(event)));
  }
}

fn spawn_system_group(
  group_name: &'static str,
  child_systems: Vec<ChildSystemContainer>,
  active: bool,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  groups.insert(group_name, SystemGroup::new());
  log::info!("🌐 Spawned system group {}", group_name);

  for tracer in tracer_txs {
    let _ = tracer.send(TraceEvent::SystemGroupSpawned { key: group_name });
  }

  for child in child_systems {
    spawn_system(
      child.to_system_container(),
      group_name,
      groups,
      base,
      app_sender.clone(),
      tracer_txs,
    );
  }

  if !active {
    deactivate_system_group(group_name, groups, base, app_sender, tracer_txs);
  }
}

fn despawn_system_group(
  group_name: &'static str,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
  tracer_txs: &TracerSenders,
) {
  if !groups.contains_key(group_name) {
    log::warn!(
      "No system group named '{}' found, cannot despawn",
      group_name
    );
    return;
  }

  let child_ids = group_child_ids(groups, group_name);
  for id in child_ids {
    let handle = SystemHandle::new(id, group_name);
    despawn_system(
      handle,
      groups,
      base,
      app_sender.clone(),
      interrupt_registry,
      tracer_txs,
    );
  }

  for tracer in tracer_txs {
    let _ = tracer.send(TraceEvent::SystemGroupDespawned { key: group_name });
  }

  let _ = groups.remove(group_name);
  log::info!("🌐 Despawned system group {}", group_name);
}

fn activate_system_group(
  group_name: &'static str,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  if !groups.contains_key(group_name) {
    log::warn!(
      "No system group named '{}' found, cannot activate",
      group_name
    );
    return;
  }

  let group = groups.get_mut(group_name).unwrap();
  if !group.active {
    group.active = true;
  } else {
    return;
  }

  let child_ids = group_child_ids(groups, group_name);
  for id in child_ids {
    let mut system = groups
      .get(group_name)
      .unwrap()
      .systems
      .get_by_id(&id)
      .unwrap();
    let ctx = SystemContext::new(
      base,
      SystemHandle::new(id, group_name),
      groups,
      app_sender.clone(),
    );

    if system.is_active(&ctx) {
      system.on_reactivate(&ctx);
    }
  }

  for tracer in tracer_txs {
    let _ = tracer.send(TraceEvent::SystemGroupActiveStateChange {
      key: group_name,
      active: true,
    });
  }
}

fn deactivate_system_group(
  group_name: &'static str,
  groups: &mut Groups,
  base: &BootSnapshot,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  if !groups.contains_key(group_name) {
    log::warn!(
      "No system group named '{}' found, cannot activate",
      group_name
    );
    return;
  }

  let group = groups.get_mut(group_name).unwrap();
  if group.active {
    group.active = false;
  } else {
    return;
  }

  let child_ids = group_child_ids(groups, group_name);
  for id in child_ids {
    let mut system = groups
      .get(group_name)
      .unwrap()
      .systems
      .get_by_id(&id)
      .unwrap();
    let ctx = SystemContext::new(
      base,
      SystemHandle::new(id, group_name),
      groups,
      app_sender.clone(),
    );
    system.on_deactivate(&ctx);
  }

  for tracer in tracer_txs {
    let _ = tracer.send(TraceEvent::SystemGroupActiveStateChange {
      key: group_name,
      active: false,
    });
  }
}

fn unregister_all_by_system(system_id: &u64, interrupt_registry: &mut EventInterruptRegistry) {
  interrupt_registry.unregister_by_system(system_id);
}

fn create_cue(
  handle: SystemHandle,
  cue_id: u64,
  cue: Cue,
  signals: Vec<Box<dyn Event>>,
  groups: &Groups,
) {
  if let Some(group) = groups.get(handle.parent_key)
    && let Some(mut system) = group.get_by_id(&handle.id)
  {
    log::info!("Creating cue for {} ({})", system.name(), handle.id);
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
  if let Some(group) = groups.get(handle.parent_key)
    && let Some(mut system) = group.get_by_id(&handle.id)
  {
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
  if let Some(group) = groups.get(handle.parent_key)
    && let Some(mut system) = group.get_by_id(&handle.id)
  {
    system.cancel_cue(cue_id);
  } else {
    log::warn!(
      "No system found with ID {}, cannot cancel cue with ID {}",
      handle.id,
      cue_id
    );
  }
}

fn group_child_ids(groups: &Groups, group_name: &'static str) -> Vec<u64> {
  groups
    .get(group_name)
    .map(|g| g.systems.values().map(|s| s.borrow().id()).collect())
    .unwrap_or(Vec::new())
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
