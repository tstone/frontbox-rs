use std::any::TypeId;
use std::collections::HashMap;

use indexmap::IndexMap;
use tokio::sync::mpsc;

use super::{RoutineDefinition, RoutineStart};
use crate::app::app_message::AppMessage;
use crate::app::app_tracer::{RoutineSkipReason, TraceEvent, TracerSenders};
use crate::app::run_loop::{apply_to_system, is_system_active};
use crate::app::{Groups, GroupsExt};
use crate::prelude::*;

/// The app's routine definitions, and the routines running right now. Owned by the run loop.
#[derive(Default)]
pub(crate) struct Routines {
  definitions: HashMap<TypeId, RoutineDefinition>,
  running: IndexMap<TypeId, RunningRoutine>,
}

impl Routines {
  pub(crate) fn new(definitions: HashMap<TypeId, RoutineDefinition>) -> Self {
    Self {
      definitions,
      running: IndexMap::new(),
    }
  }

  pub(crate) fn start(
    &mut self,
    start: RoutineStart,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let RoutineStart {
      origin,
      type_id,
      name,
      payload,
      into_complete,
    } = start;

    if let Some(existing) = self.running.get(&type_id) {
      log::warn!(
        target: "frontbox::routines",
        "Routine {} was started by {} while already running (started by {}); ignoring the second start",
        name,
        Self::describe(groups, origin),
        Self::describe(groups, existing.origin),
      );
      tracer_txs.send(TraceEvent::RoutineRejected {
        routine: name,
        origin: origin.id,
        running_origin: existing.origin.id,
      });
      return;
    }

    if !self.definitions.contains_key(&type_id) {
      log::debug!(target: "frontbox::routines", "Routine {} has no definition, so it finishes immediately", name);
    }

    log::debug!(target: "frontbox::routines", "Routine {} started by {}", name, Self::describe(groups, origin));
    self.running.insert(
      type_id,
      RunningRoutine {
        name,
        origin,
        payload,
        into_complete,
        next: 0,
        current: None,
        paused: false,
      },
    );
    tracer_txs.send(TraceEvent::RoutineStarted {
      routine: name,
      origin: origin.id,
    });

    self.advance(type_id, groups, base, app_sender, tracer_txs);
  }

  pub(crate) fn complete_step(
    &mut self,
    caller: SystemHandle,
    type_id: TypeId,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let Some(routine) = self.running.get_mut(&type_id) else {
      log::debug!(
        target: "frontbox::routines",
        "Ignoring a routine step completion from system {}: that routine isn't running",
        caller.id
      );
      return;
    };

    if !routine.current.is_some_and(|current| current == caller) {
      log::debug!(
        target: "frontbox::routines",
        "Ignoring a step completion of routine {} from system {}: it isn't the current participant",
        routine.name,
        caller.id
      );
      return;
    }

    routine.current = None;
    self.advance(type_id, groups, base, app_sender, tracer_txs);
  }

  /// The systems were despawned, or their activity changed. Any routine they're the origin or current participant of
  /// looks again at where it stands: a step whose participant stopped is over, and a paused routine whose origin is
  /// active again resumes.
  pub(crate) fn system_activation_changed(
    &mut self,
    handles: &[SystemHandle],
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let involves = |routine: &RunningRoutine| {
      handles.iter().any(|&handle| {
        routine.origin == handle || routine.current.is_some_and(|current| current == handle)
      })
    };
    let affected: Vec<TypeId> = self
      .running
      .iter()
      .filter(|(_, routine)| involves(routine))
      .map(|(type_id, _)| *type_id)
      .collect();

    for type_id in affected {
      let Some(routine) = self.running.get_mut(&type_id) else {
        continue;
      };

      if let Some(current) = routine.current {
        if is_system_active(current, groups, base, app_sender, tracer_txs) {
          continue;
        }
        log::debug!(
          target: "frontbox::routines",
          "Routine {}: system {} stopped mid-step, so its step counts as complete",
          routine.name,
          current.id
        );
        routine.current = None;
      }

      self.advance(type_id, groups, base, app_sender, tracer_txs);
    }
  }

  /// Hand the routine to its next active participant, or finish it if there are none left. Does nothing while the
  /// origin is inactive.
  fn advance(
    &mut self,
    type_id: TypeId,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    loop {
      let Some(routine) = self.running.get_mut(&type_id) else {
        return;
      };

      match Self::origin_state(routine.origin, groups, base, app_sender, tracer_txs) {
        OriginState::Despawned => {
          log::warn!(
            target: "frontbox::routines",
            "Routine {} abandoned: its origin (system {}) was despawned",
            routine.name,
            routine.origin.id
          );
          tracer_txs.send(TraceEvent::RoutineAbandoned {
            routine: routine.name,
            origin: routine.origin.id,
          });
          self.running.shift_remove(&type_id);
          return;
        }
        OriginState::Inactive => {
          if !routine.paused {
            log::debug!(target: "frontbox::routines", "Routine {} paused: its origin is inactive", routine.name);
            routine.paused = true;
            tracer_txs.send(TraceEvent::RoutinePaused {
              routine: routine.name,
            });
          }
          return;
        }
        OriginState::Active => {
          if routine.paused {
            log::debug!(target: "frontbox::routines", "Routine {} resumed", routine.name);
            routine.paused = false;
            tracer_txs.send(TraceEvent::RoutineResumed {
              routine: routine.name,
            });
          }
        }
      }

      let next = self
        .definitions
        .get(&type_id)
        .and_then(|definition| definition.participant(routine.next));
      routine.next += 1;

      let Some(participant) = next else {
        self.finish(type_id, groups, base, app_sender, tracer_txs);
        return;
      };

      let Some(handle) = groups.nearest(routine.origin.parent_key, participant.type_id) else {
        log::debug!(
          target: "frontbox::routines",
          "Routine {}: skipping {}, which has no instance in reach of the origin",
          routine.name,
          participant.name
        );
        tracer_txs.send(TraceEvent::RoutineSkipped {
          routine: routine.name,
          participant: participant.name,
          reason: RoutineSkipReason::Missing,
        });
        continue;
      };

      if !is_system_active(handle, groups, base, app_sender, tracer_txs) {
        log::debug!(
          target: "frontbox::routines",
          "Routine {}: skipping {}, which is inactive",
          routine.name,
          participant.name
        );
        tracer_txs.send(TraceEvent::RoutineSkipped {
          routine: routine.name,
          participant: participant.name,
          reason: RoutineSkipReason::Inactive,
        });
        continue;
      }

      log::debug!(target: "frontbox::routines", "Routine {}: handing to {}", routine.name, participant.name);
      routine.current = Some(handle);
      tracer_txs.send(TraceEvent::RoutineStep {
        routine: routine.name,
        participant: handle.id,
        group: handle.parent_key,
      });

      // completing only sends a message, so the participant can't re-enter the routine from here
      let payload = routine.payload.as_ref();
      apply_to_system(
        handle,
        groups,
        base,
        app_sender,
        tracer_txs,
        |system, ctx| {
          system.on_routine(payload, ctx);
        },
      );
      return;
    }
  }

  /// Only called once the origin is known to be active
  fn finish(
    &mut self,
    type_id: TypeId,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let Some(routine) = self.running.shift_remove(&type_id) else {
      return;
    };

    log::debug!(target: "frontbox::routines", "Routine {} finished", routine.name);
    tracer_txs.send(TraceEvent::RoutineFinished {
      routine: routine.name,
      origin: routine.origin.id,
    });

    let complete = (routine.into_complete)(routine.payload);
    apply_to_system(
      routine.origin,
      groups,
      base,
      app_sender,
      tracer_txs,
      |system, ctx| {
        system.on_event(complete.as_ref(), ctx);
      },
    );
  }

  fn origin_state(
    origin: SystemHandle,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) -> OriginState {
    if !groups.contains_system(origin) {
      OriginState::Despawned
    } else if is_system_active(origin, groups, base, app_sender, tracer_txs) {
      OriginState::Active
    } else {
      OriginState::Inactive
    }
  }

  /// The system's name for logs, or its id once it's gone
  fn describe(groups: &Groups, handle: SystemHandle) -> String {
    groups
      .system_name(handle)
      .map(str::to_string)
      .unwrap_or_else(|| format!("system {}", handle.id))
  }

  #[cfg(test)]
  pub(super) fn is_running(&self, type_id: TypeId) -> bool {
    self.running.contains_key(&type_id)
  }
}

struct RunningRoutine {
  name: &'static str,
  origin: SystemHandle,
  payload: Box<dyn Event>,
  /// Wraps the payload in its `RoutineComplete`
  into_complete: fn(Box<dyn Event>) -> Box<dyn Event>,
  /// Index of the next participant in the definition
  next: usize,
  /// The participant whose step is in progress
  current: Option<SystemHandle>,
  /// Waiting for the origin to be active again
  paused: bool,
}

enum OriginState {
  Active,
  Inactive,
  Despawned,
}
