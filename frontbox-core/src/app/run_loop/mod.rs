//! The app's main loop
//!
//! - `delivery`: handing events and handler calls to systems, respecting their activity
//! - `harness`: driving the loop synchronously, for tests (with the `testing` feature, or in this crate's tests)
//! - `lifecycle`: spawning, despawning, activating and deactivating systems and groups
//! - `switches`: switch state changes, from the machine or a tracer
//! - `timing`: ticks, the tick timer and cues

mod delivery;
#[cfg(any(test, feature = "testing"))]
pub(crate) mod harness;
mod lifecycle;
mod switches;
#[cfg(test)]
mod test_support;
mod timing;

use fast_protocol::SwitchState;
use std::any::TypeId;
use std::ops::ControlFlow;
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

pub(crate) use delivery::{apply_to_system, apply_to_systems, emit_event_to_origin, emit_event_to_others, is_system_active};

use delivery::emit_activity_changed;
use timing::{spawn_system_ticker, tick_delta};

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
      AppMessage::MachineDisconnected => {
        self.emit_event(Some(self.machine_system_id), EventBox::new(MachineDisconnected));
      }
      AppMessage::MachineReconnected(switch_states) => {
        // switches that changed while disconnected are taken as they are now, without events for the changes
        self.base.switches.update_switch_states(switch_states);
        self.emit_event(Some(self.machine_system_id), EventBox::new(MachineReconnected));
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

  /// Despawn every system, then close the metrics store and, if asked, shut down the OS
  async fn shutdown(self, scope: ShutdownScope) {
    self.despawn_all();

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

  /// Call `on_despawn` on every system, regardless of whether it was active, in reverse spawn order. The systems
  /// stay in place: the loop is about to end, so nothing else will be handed to them.
  fn despawn_all(&self) {
    for (key, group) in self.groups.iter().rev() {
      for cell in group.values().rev() {
        let mut system = cell.borrow_mut();
        let ctx = SystemContext::new(
          &self.base,
          SystemHandle::new(system.id(), *key),
          &self.groups,
          &self.app_sender,
        );
        system.catch_panic(|system| system.on_despawn(&ctx));
      }
    }
  }

  /// Tell routines that these systems were despawned or may have changed activity
  fn activation_changed(&mut self, handles: &[SystemHandle]) {
    self
      .routines
      .system_activation_changed(handles, &self.groups, &self.base, &self.app_sender, &self.tracers);
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
}

/// `Synchronize` is a special event recognized by Frontbox to re-start the internal loop in cadence with receiving this event.
#[derive(serde::Serialize, Event, PartialEq, Eq)]
pub struct Synchronize;

static SYNCHRONIZE_TYPE_ID: TypeId = TypeId::of::<Synchronize>();

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::run_loop::test_support::*;

  #[test]
  fn a_shutdown_message_stops_the_loop() {
    let mut h = Harness::new();

    let flow = h.handle(AppMessage::Shutdown(ShutdownScope::OperatingSystem));

    assert_eq!(flow, ControlFlow::Break(ShutdownScope::OperatingSystem));
  }

  #[test]
  fn shutting_down_despawns_every_system_in_reverse_including_inactive_ones() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Probe::<1>(log.clone()));
    h.spawn_group_of(owner, "group", vec![Probe::<2>(log.clone()).into()], false);
    h.spawn(Probe::<3>(log.clone()));
    log.take();

    h.despawn_all();

    // groups last spawned first, and within each group, systems last spawned first
    assert_eq!(log.take(), ["2 on_despawn", "3 on_despawn", "1 on_despawn"]);
  }

  #[test]
  fn a_config_change_is_broadcast() {
    let mut h = Harness::new();
    let heard = h.listen();

    h.send(AppMessage::ConfigChanged("volume"));
    h.settle();

    assert_eq!(heard.take(), [Heard::ConfigChanged("volume")]);
  }
}
