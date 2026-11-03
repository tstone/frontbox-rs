use std::any::TypeId;
use std::collections::HashMap;

use indexmap::IndexMap;
use tokio::sync::mpsc;

use super::{RoutineDefinition, RoutineStart};
use crate::app::app_message::AppMessage;
use crate::app::app_tracer::{RoutineSkipReason, TraceEvent, TracerSenders};
use crate::app::event_box::EventBox;
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

  /// Registers the routine, returning its `RoutineStarted` for the run loop to emit. Its first participant isn't handed
  /// the routine until `begin`, which the run loop queues after emitting it, so whatever the listeners of
  /// `RoutineStarted` emit lands before the first step.
  pub(crate) fn start(
    &mut self,
    start: RoutineStart,
    groups: &Groups,
    tracer_txs: &TracerSenders,
  ) -> Option<EventBox> {
    let RoutineStart {
      origin,
      scope,
      type_id,
      name,
      routine: event,
      into_started,
      into_completed,
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
      return None;
    }

    if !self.definitions.contains_key(&type_id) {
      log::debug!(target: "frontbox::routines", "Routine {} has no definition, so it completes immediately", name);
    }

    log::debug!(target: "frontbox::routines", "Routine {} started by {}", name, Self::describe(groups, origin));
    let started = into_started(event.as_ref());
    self.running.insert(
      type_id,
      RunningRoutine {
        name,
        origin,
        scope,
        routine: event,
        into_completed,
        begun: false,
        next: 0,
        current: None,
        paused: false,
        halted: false,
      },
    );
    tracer_txs.send(TraceEvent::RoutineStarted {
      routine: name,
      origin: origin.id,
    });

    Some(started)
  }

  /// Hand a started routine to its first participant
  pub(crate) fn begin(
    &mut self,
    type_id: TypeId,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let Some(routine) = self.running.get_mut(&type_id) else {
      return;
    };
    if routine.begun {
      return;
    }
    routine.begun = true;
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
    if self.end_step(caller, type_id, "step completion").is_some() {
      self.advance(type_id, groups, base, app_sender, tracer_txs);
    }
  }

  /// The current participant ends the routine with its step. It completes as halted once its origin is active.
  pub(crate) fn halt(
    &mut self,
    caller: SystemHandle,
    type_id: TypeId,
    groups: &Groups,
    base: &AppState,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let Some(routine) = self.end_step(caller, type_id, "halt") else {
      return;
    };

    log::debug!(
      target: "frontbox::routines",
      "Routine {} halted by {}",
      routine.name,
      Self::describe(groups, caller)
    );
    routine.halted = true;
    tracer_txs.send(TraceEvent::RoutineHalted {
      routine: routine.name,
      participant: caller.id,
      group: caller.group,
    });
    self.advance(type_id, groups, base, app_sender, tracer_txs);
  }

  /// Ends the caller's step, if it's the current participant of the routine
  fn end_step(&mut self, caller: SystemHandle, type_id: TypeId, what: &str) -> Option<&mut RunningRoutine> {
    let Some(routine) = self.running.get_mut(&type_id) else {
      log::debug!(
        target: "frontbox::routines",
        "Ignoring a routine {} from system {}: that routine isn't running",
        what,
        caller.id
      );
      return None;
    };

    if !routine.current.is_some_and(|current| current == caller) {
      log::debug!(
        target: "frontbox::routines",
        "Ignoring a {} of routine {} from system {}: it isn't the current participant",
        what,
        routine.name,
        caller.id
      );
      return None;
    }

    routine.current = None;
    Some(routine)
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
    // one that hasn't begun is left to `begin`, which sees where the origin stands then
    let involves = |routine: &RunningRoutine| {
      routine.begun
        && handles.iter().any(|&handle| {
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

  /// Hand the routine to its next active participant, or complete it if there are none left. Does nothing while the
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

      if routine.halted {
        self.complete(type_id, RoutineCompletion::Halted, app_sender, tracer_txs);
        return;
      }

      let next = self
        .definitions
        .get(&type_id)
        .and_then(|definition| definition.participant(routine.next));
      routine.next += 1;

      let Some(participant) = next else {
        self.complete(type_id, RoutineCompletion::AllSteps, app_sender, tracer_txs);
        return;
      };

      let Some(handle) = groups.nearest(routine.scope, participant.type_id) else {
        log::debug!(
          target: "frontbox::routines",
          "Routine {}: skipping {}, which has no instance in reach of the routine's group",
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
        group: handle.group,
      });

      // completing only sends a message, so the participant can't re-enter the routine from here
      let event = routine.routine.as_ref();
      apply_to_system(
        handle,
        groups,
        base,
        app_sender,
        tracer_txs,
        |system, ctx| {
          system.on_routine(event, ctx);
        },
      );
      return;
    }
  }

  /// Only called once the origin is known to be active. `RoutineCompleted` is queued rather than delivered here, so it
  /// lands after whatever was queued before it. The origin gets it first, and everyone else after whatever the origin
  /// queued in response, so a system it activates by carrying on (e.g. a state change) hears about it.
  fn complete(
    &mut self,
    type_id: TypeId,
    completion: RoutineCompletion,
    app_sender: &mpsc::UnboundedSender<AppMessage>,
    tracer_txs: &TracerSenders,
  ) {
    let Some(routine) = self.running.shift_remove(&type_id) else {
      return;
    };

    log::debug!(target: "frontbox::routines", "Routine {} completed ({:?})", routine.name, completion);
    tracer_txs.send(TraceEvent::RoutineCompleted {
      routine: routine.name,
      origin: routine.origin.id,
      completion,
    });

    let completed = (routine.into_completed)(routine.routine, completion);
    let _ = app_sender.send(AppMessage::EmitEventOriginFirst(routine.origin, completed));
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
  /// The group participants are looked up from
  scope: GroupId,
  /// The routine as started, which participants are handed
  routine: Box<dyn Event>,
  /// Wraps the routine in its `RoutineCompleted`
  into_completed: fn(Box<dyn Event>, RoutineCompletion) -> EventBox,
  /// `RoutineStarted` has gone out, and the first participant was handed the routine
  begun: bool,
  /// Index of the next participant in the definition
  next: usize,
  /// The participant whose step is in progress
  current: Option<SystemHandle>,
  /// Waiting for the origin to be active again
  paused: bool,
  /// A participant halted it, so it completes without handing it to anyone else
  halted: bool,
}

enum OriginState {
  Active,
  Inactive,
  Despawned,
}
