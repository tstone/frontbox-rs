use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use tokio::sync::mpsc;

use super::*;
use crate::app::app_message::AppMessage;
use crate::app::app_tracer::TracerSenders;
use crate::app::run_loop::{apply_to_system, apply_to_systems};
use crate::app::{Groups, GroupsExt};
use crate::prelude::*;

/// What the test systems did, in order, so tests can check the sequence across systems
type Log = Rc<RefCell<Vec<String>>>;

#[derive(Serialize, Event, Clone)]
struct TurnEnding;

/// A participant that completes its step right away, or waits to be told
struct Step<const N: usize> {
  log: Log,
  complete_now: bool,
  /// The name of the group it was spawned into
  group: &'static str,
}

impl<const N: usize> System for Step<N> {
  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    if routine.is::<TurnEnding>() {
      self
        .log
        .borrow_mut()
        .push(format!("step {N} in {}", self.group));
      if self.complete_now {
        ctx.complete_routine_step::<TurnEnding>();
      }
    }
  }
}

/// A participant that waits, and whose activity the test controls
struct Toggled {
  active: Rc<Cell<bool>>,
}

impl System for Toggled {
  fn on_routine(&mut self, _routine: &dyn Event, _ctx: &SystemContext) {}

  fn is_active(&self, _ctx: &SystemContext) -> bool {
    self.active.get()
  }
}

struct Origin {
  log: Log,
}

impl System for Origin {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    log_completion(&self.log, event);
  }
}

/// Logs "completed", or "halted" for a halted routine
fn log_completion(log: &Log, event: &dyn Event) {
  if let Some(done) = event.downcast_ref::<RoutineCompleted<TurnEnding>>() {
    let entry = match done.completion {
      RoutineCompletion::AllSteps => "completed",
      RoutineCompletion::Halted => "halted",
    };
    log.borrow_mut().push(entry.to_string());
  }
}

/// An origin whose activity the test controls
struct OriginToggled {
  log: Log,
  active: Rc<Cell<bool>>,
}

impl System for OriginToggled {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    log_completion(&self.log, event);
  }

  fn is_active(&self, _ctx: &SystemContext) -> bool {
    self.active.get()
  }
}

#[derive(Serialize, Event)]
struct CleanedUp;

/// A system that isn't a participant, watching the routine's events. Optionally emits `CleanedUp` when it starts.
struct Watcher {
  log: Log,
  clean_up: bool,
}

impl System for Watcher {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<RoutineStarted<TurnEnding>>() {
      self.log.borrow_mut().push("saw started".to_string());
      if self.clean_up {
        ctx.emit(CleanedUp);
      }
    } else if event.is::<CleanedUp>() {
      self.log.borrow_mut().push("cleaned up".to_string());
    } else if event.is::<RoutineCompleted<TurnEnding>>() {
      self.log.borrow_mut().push("saw completed".to_string());
    }
  }
}

/// A system that leaves `on_routine` alone
struct Bystander;

impl System for Bystander {}

struct Harness {
  base: AppState,
  groups: Groups,
  routines: Routines,
  tx: mpsc::UnboundedSender<AppMessage>,
  rx: mpsc::UnboundedReceiver<AppMessage>,
  tracers: TracerSenders,
  log: Log,
  /// Groups by name, so tests can refer to them that way
  group_ids: HashMap<&'static str, GroupId>,
}

impl Harness {
  fn new(definition: RoutineDefinition) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut groups = Groups::new();
    groups.insert(ROOT_GROUP, SystemGroup::new(ROOT_GROUP_NAME));

    Self {
      base: AppState::default(),
      groups,
      routines: Routines::new(HashMap::from([(TypeId::of::<TurnEnding>(), definition)])),
      tx,
      rx,
      tracers: TracerSenders::new(Vec::new()),
      log: Rc::default(),
      group_ids: HashMap::from([(ROOT_GROUP_NAME, ROOT_GROUP)]),
    }
  }

  /// The group with this name, spawned if there isn't one yet
  fn group(&mut self, name: &'static str) -> GroupId {
    if let Some(id) = self.group_ids.get(name) {
      return *id;
    }
    let id = GroupId::next();
    self.groups.insert(id, SystemGroup::new(name));
    self.group_ids.insert(name, id);
    id
  }

  fn spawn(&mut self, group: &'static str, system: impl System + 'static) -> SystemHandle {
    let id = self.group(group);
    let container = SystemContainer::new(system);
    let handle = SystemHandle::new(container.id(), id);
    self.groups[&id].insert(container);
    handle
  }

  fn step<const N: usize>(&mut self, group: &'static str, complete_now: bool) -> SystemHandle {
    let log = self.log.clone();
    self.spawn(
      group,
      Step::<N> {
        log,
        complete_now,
        group,
      },
    )
  }

  fn origin(&mut self, group: &'static str) -> SystemHandle {
    let log = self.log.clone();
    self.spawn(group, Origin { log })
  }

  fn watcher(&mut self, clean_up: bool) -> SystemHandle {
    let log = self.log.clone();
    self.spawn(ROOT_GROUP_NAME, Watcher { log, clean_up })
  }

  fn ctx(&self, handle: SystemHandle) -> SystemContext<'_> {
    SystemContext::new(&self.base, handle, &self.groups, self.tx.clone())
  }

  /// Process routine messages and events the way the run loop does, until there are none left
  fn pump(&mut self) {
    while let Ok(message) = self.rx.try_recv() {
      match message {
        AppMessage::StartRoutine(start) => {
          let type_id = start.type_id;
          if let Some(started) = self.routines.start(start, &self.groups, &self.tracers) {
            self.broadcast(started.event.as_ref());
            let _ = self.tx.send(AppMessage::BeginRoutine(type_id));
          }
        }
        AppMessage::BeginRoutine(type_id) => {
          self
            .routines
            .begin(type_id, &self.groups, &self.base, &self.tx, &self.tracers)
        }
        AppMessage::EmitEvent(_, event) => self.broadcast(event.event.as_ref()),
        AppMessage::CompleteRoutineStep(caller, type_id) => self.routines.complete_step(
          caller,
          type_id,
          &self.groups,
          &self.base,
          &self.tx,
          &self.tracers,
        ),
        AppMessage::HaltRoutine(caller, type_id) => self.routines.halt(
          caller,
          type_id,
          &self.groups,
          &self.base,
          &self.tx,
          &self.tracers,
        ),
        AppMessage::SystemActivityChanged(handle) => self.routines.system_activation_changed(
          &[handle],
          &self.groups,
          &self.base,
          &self.tx,
          &self.tracers,
        ),
        _ => {}
      }
    }
  }

  fn broadcast(&self, event: &dyn Event) {
    apply_to_systems(
      &self.groups,
      &self.base,
      &self.tx,
      &self.tracers,
      |system, ctx| {
        system.on_event(event, ctx);
      },
    );
  }

  /// The run loop tells routines about a change it made itself, after making it
  fn changed(&mut self, handles: &[SystemHandle]) {
    self.routines.system_activation_changed(
      handles,
      &self.groups,
      &self.base,
      &self.tx,
      &self.tracers,
    );
    self.pump();
  }

  /// The framework checks a system's `is_active` whenever it touches it, e.g. on each tick
  fn touch(&mut self, handle: SystemHandle) {
    apply_to_system(
      handle,
      &self.groups,
      &self.base,
      &self.tx,
      &self.tracers,
      |_, _| (),
    );
    self.pump();
  }

  fn despawn(&mut self, handle: SystemHandle) {
    self
      .groups
      .get_mut(&handle.group)
      .unwrap()
      .remove(handle.id);
    self.changed(&[handle]);
  }

  fn start(&mut self, origin: SystemHandle) {
    self.ctx(origin).start_routine(TurnEnding);
    self.pump();
  }

  fn complete(&mut self, participant: SystemHandle) {
    self.ctx(participant).complete_routine_step::<TurnEnding>();
    self.pump();
  }

  fn halt(&mut self, participant: SystemHandle) {
    self.ctx(participant).halt_routine::<TurnEnding>();
    self.pump();
  }

  fn set_group_active(&mut self, name: &'static str, active: bool) {
    let key = self.group(name);
    let group = self.groups.get_mut(&key).unwrap();
    if active {
      group.activate()
    } else {
      group.deactivate()
    }
    let handles = self.groups.handles_in(key);
    self.changed(&handles);
  }

  fn is_running(&self) -> bool {
    self.routines.is_running(TypeId::of::<TurnEnding>())
  }

  fn log(&self) -> Vec<String> {
    self.log.borrow().clone()
  }
}

#[test]
fn runs_participants_in_order_then_signals_the_origin() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.step::<2>(ROOT_GROUP_NAME, true);
  h.step::<1>(ROOT_GROUP_NAME, true);

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in root", "step 2 in root", "completed"]);
  assert!(!h.is_running());
}

#[test]
fn waits_for_a_later_completion() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  assert_eq!(h.log(), ["step 1 in root"]);

  h.complete(first);
  assert_eq!(h.log(), ["step 1 in root", "step 2 in root", "completed"]);
}

#[test]
fn empty_and_undefined_routines_complete_immediately() {
  let mut h = Harness::new(routine![]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.start(origin);
  assert_eq!(h.log(), ["completed"]);

  let mut h = Harness::new(routine![]);
  h.routines = Routines::default();
  let origin = h.origin(ROOT_GROUP_NAME);
  h.start(origin);
  assert_eq!(h.log(), ["completed"]);
}

#[test]
fn a_second_start_is_ignored_while_running() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let step = h.step::<1>(ROOT_GROUP_NAME, false);

  h.start(origin);
  h.start(origin);
  assert_eq!(h.log(), ["step 1 in root"]);

  h.complete(step);
  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn nearest_instance_wins() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  h.step::<1>(ROOT_GROUP_NAME, true);
  h.step::<1>("player", true);
  h.step::<2>(ROOT_GROUP_NAME, true);
  let origin = h.origin("player");

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in player", "step 2 in root", "completed"]);
}

#[test]
fn participants_out_of_reach_are_skipped() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  h.step::<1>("player", true);
  h.step::<2>(ROOT_GROUP_NAME, true);
  let origin = h.origin(ROOT_GROUP_NAME);

  h.start(origin);

  assert_eq!(h.log(), ["step 2 in root", "completed"]);
}

#[test]
fn started_in_a_group_reaches_its_participants() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  h.step::<1>("player", true);
  h.step::<2>(ROOT_GROUP_NAME, true);
  let origin = h.origin(ROOT_GROUP_NAME);
  let player = GroupHandle::new(h.group("player"), "player");

  h.ctx(origin).start_routine_in(&player, TurnEnding);
  h.pump();

  assert_eq!(h.log(), ["step 1 in player", "step 2 in root", "completed"]);
}

#[test]
fn inactive_participants_are_skipped() {
  let mut h = Harness::new(routine![Toggled, Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.spawn(
    ROOT_GROUP_NAME,
    Toggled {
      active: Rc::new(Cell::new(false)),
    },
  );
  h.step::<1>(ROOT_GROUP_NAME, true);

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn only_the_current_participant_can_complete() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  let second = h.step::<2>(ROOT_GROUP_NAME, false);

  h.start(origin);
  h.complete(second);
  h.complete(origin);
  assert_eq!(h.log(), ["step 1 in root"]);

  // a duplicate completion from the first step doesn't skip the second
  h.ctx(first).complete_routine_step::<TurnEnding>();
  h.ctx(first).complete_routine_step::<TurnEnding>();
  h.pump();
  assert_eq!(h.log(), ["step 1 in root", "step 2 in root"]);
}

#[test]
fn a_participant_despawned_mid_step_counts_as_complete() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  h.despawn(first);

  assert_eq!(h.log(), ["step 1 in root", "step 2 in root", "completed"]);
}

#[test]
fn a_participant_deactivated_mid_step_counts_as_complete() {
  let mut h = Harness::new(routine![Toggled, Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let active = Rc::new(Cell::new(true));
  h.spawn(
    ROOT_GROUP_NAME,
    Toggled {
      active: active.clone(),
    },
  );
  let toggled = h.groups[&ROOT_GROUP].get_id::<Toggled>().unwrap();
  h.step::<1>(ROOT_GROUP_NAME, true);

  h.start(origin);
  assert!(h.log().is_empty());

  // nothing happens until the framework next checks the system
  active.set(false);
  assert!(h.log().is_empty());
  h.touch(SystemHandle::new(toggled, ROOT_GROUP));

  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn a_group_deactivated_and_reactivated_in_a_row_still_ends_the_step() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin("player");
  h.step::<1>("player", false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  // a poll in between would never see the group inactive
  h.set_group_active("player", false);
  h.set_group_active("player", true);

  assert_eq!(h.log(), ["step 1 in player", "step 2 in root", "completed"]);
}

#[test]
fn copies_in_other_groups_dont_affect_the_step() {
  #[derive(Clone)]
  struct Waiting;

  impl System for Waiting {
    fn on_routine(&mut self, _routine: &dyn Event, _ctx: &SystemContext) {}
  }

  let mut h = Harness::new(routine![Waiting, Step<1>]);
  // copies of a child system keep its id, as when one is spawned into each player's group
  let template = ChildSystemContainer::new(Waiting);
  let player1 = h.group("player 1");
  let player2 = h.group("player 2");
  for group in [player1, player2] {
    let container = template.clone().to_system_container();
    h.groups[&group].insert(container);
  }
  let id = *h.groups[&player1].ids()[0];
  h.step::<1>(ROOT_GROUP_NAME, true);
  let origin = h.origin("player 2");

  h.start(origin);
  h.set_group_active("player 1", false);
  h.complete(SystemHandle::new(id, player1));
  assert!(h.log().is_empty());

  h.complete(SystemHandle::new(id, player2));
  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn pauses_while_the_origin_is_inactive() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin("player");
  h.step::<1>("player", false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  // the step in progress counts as complete once its participant is inactive, but the routine goes no further
  h.set_group_active("player", false);
  assert_eq!(h.log(), ["step 1 in player"]);
  assert!(h.is_running());

  h.set_group_active("player", true);
  assert_eq!(h.log(), ["step 1 in player", "step 2 in root", "completed"]);
}

#[test]
fn the_signal_waits_for_the_origin() {
  let mut h = Harness::new(routine![Step<1>]);
  let active = Rc::new(Cell::new(true));
  let origin = h.spawn(
    ROOT_GROUP_NAME,
    OriginToggled {
      log: h.log.clone(),
      active: active.clone(),
    },
  );
  let step = h.step::<1>(ROOT_GROUP_NAME, false);

  h.start(origin);
  active.set(false);
  h.complete(step);
  assert_eq!(h.log(), ["step 1 in root"]);
  assert!(h.is_running());

  active.set(true);
  h.touch(origin);
  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn abandoned_when_the_origin_is_despawned() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  h.despawn(origin);
  h.complete(first);

  assert_eq!(h.log(), ["step 1 in root"]);
  assert!(!h.is_running());
}

#[test]
fn the_origin_gets_the_routine_back() {
  #[derive(Serialize, Event, Clone)]
  struct Drained {
    balls: u8,
  }

  struct Counter {
    log: Log,
  }

  impl System for Counter {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if let Some(done) = event.downcast_ref::<RoutineCompleted<Drained>>() {
        self
          .log
          .borrow_mut()
          .push(format!("{} balls", done.routine.balls));
      }
    }
  }

  let mut h = Harness::new(routine![]);
  let log = h.log.clone();
  let origin = h.spawn(ROOT_GROUP_NAME, Counter { log });

  h.ctx(origin).start_routine(Drained { balls: 3 });
  h.pump();

  assert_eq!(h.log(), ["3 balls"]);
}

#[test]
fn test_context_records_starts_and_completions() {
  let mut test_ctx = TestContext::default();

  let mut step = Step::<1> {
    log: Rc::default(),
    complete_now: true,
    group: ROOT_GROUP_NAME,
  };
  step.on_routine(&TurnEnding, &test_ctx.sys_ctx());
  assert!(test_ctx.routine_step_completed::<TurnEnding>());

  test_ctx.sys_ctx().start_routine(TurnEnding);
  let started = test_ctx.routines_started();
  assert_eq!(started.len(), 1);
  assert!(started[0].as_ref().is::<TurnEnding>());
}

#[test]
fn participants_without_on_routine_complete() {
  let mut h = Harness::new(routine![Bystander, Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.spawn(ROOT_GROUP_NAME, Bystander);
  h.step::<1>(ROOT_GROUP_NAME, true);

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in root", "completed"]);
}

#[test]
fn started_and_completed_are_broadcast_around_the_steps() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.watcher(false);
  let step = h.step::<1>(ROOT_GROUP_NAME, false);

  h.start(origin);
  assert_eq!(h.log(), ["saw started", "step 1 in root"]);

  h.complete(step);
  assert_eq!(
    h.log(),
    [
      "saw started",
      "step 1 in root",
      "completed",
      "saw completed"
    ]
  );
}

#[test]
fn what_listeners_emit_on_started_lands_before_the_first_step() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.watcher(true);
  h.step::<1>(ROOT_GROUP_NAME, true);

  h.start(origin);

  assert_eq!(
    h.log(),
    [
      "saw started",
      "cleaned up",
      "step 1 in root",
      "completed",
      "saw completed"
    ]
  );
}

#[test]
fn empty_and_undefined_routines_still_broadcast_both_events() {
  let mut h = Harness::new(routine![]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.watcher(false);
  h.start(origin);
  assert_eq!(h.log(), ["saw started", "completed", "saw completed"]);

  let mut h = Harness::new(routine![]);
  h.routines = Routines::default();
  let origin = h.origin(ROOT_GROUP_NAME);
  h.watcher(false);
  h.start(origin);
  assert_eq!(h.log(), ["saw started", "completed", "saw completed"]);
}

#[test]
fn a_rejected_start_broadcasts_nothing() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.step::<1>(ROOT_GROUP_NAME, false);
  h.watcher(false);

  h.start(origin);
  h.start(origin);

  assert_eq!(h.log(), ["saw started", "step 1 in root"]);
}

#[test]
fn an_abandoned_routine_never_completes() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let step = h.step::<1>(ROOT_GROUP_NAME, false);
  h.watcher(false);

  h.start(origin);
  h.despawn(origin);
  h.complete(step);

  assert_eq!(h.log(), ["saw started", "step 1 in root"]);
}

#[test]
fn a_halt_skips_the_remaining_participants() {
  let mut h = Harness::new(routine![Step<1>, Step<2>, Step<3>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  h.step::<1>(ROOT_GROUP_NAME, true);
  let second = h.step::<2>(ROOT_GROUP_NAME, false);
  h.step::<3>(ROOT_GROUP_NAME, true);

  h.start(origin);
  h.halt(second);

  assert_eq!(h.log(), ["step 1 in root", "step 2 in root", "halted"]);
  assert!(!h.is_running());
}

#[test]
fn only_the_current_participant_can_halt() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP_NAME);
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  let second = h.step::<2>(ROOT_GROUP_NAME, false);

  h.start(origin);
  h.halt(second);
  h.halt(origin);
  assert_eq!(h.log(), ["step 1 in root"]);

  h.complete(first);
  assert_eq!(h.log(), ["step 1 in root", "step 2 in root"]);
}

#[test]
fn a_halt_waits_for_the_origin() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let active = Rc::new(Cell::new(true));
  let origin = h.spawn(
    ROOT_GROUP_NAME,
    OriginToggled {
      log: h.log.clone(),
      active: active.clone(),
    },
  );
  let first = h.step::<1>(ROOT_GROUP_NAME, false);
  h.step::<2>(ROOT_GROUP_NAME, true);

  h.start(origin);
  active.set(false);
  h.halt(first);
  assert_eq!(h.log(), ["step 1 in root"]);
  assert!(h.is_running());

  // still halted once the origin is back: the next participant isn't handed the routine
  active.set(true);
  h.touch(origin);
  assert_eq!(h.log(), ["step 1 in root", "halted"]);
}

#[test]
fn test_context_records_halts() {
  let mut test_ctx = TestContext::default();
  test_ctx.sys_ctx().halt_routine::<TurnEnding>();
  assert!(test_ctx.routine_halted::<TurnEnding>());
}
