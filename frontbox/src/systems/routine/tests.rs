use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use tokio::sync::mpsc;

use super::*;
use crate::app::app_message::AppMessage;
use crate::app::app_tracer::TracerSenders;
use crate::app::run_loop::apply_to_system;
use crate::app::{Groups, GroupsExt};
use crate::prelude::*;

/// What the test systems did, in order, so tests can check the sequence across systems
type Log = Rc<RefCell<Vec<String>>>;

#[derive(Serialize, Event)]
struct EndOfTurn;

/// A participant that completes its step right away, or waits to be told
struct Step<const N: usize> {
  log: Log,
  complete_now: bool,
}

impl<const N: usize> System for Step<N> {
  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    if routine.is::<EndOfTurn>() {
      self
        .log
        .borrow_mut()
        .push(format!("step {N} in {}", ctx.current_handle().parent_key));
      if self.complete_now {
        ctx.complete_routine_step::<EndOfTurn>();
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
    if event.is::<RoutineComplete<EndOfTurn>>() {
      self.log.borrow_mut().push("finished".to_string());
    }
  }
}

/// An origin whose activity the test controls
struct OriginToggled {
  log: Log,
  active: Rc<Cell<bool>>,
}

impl System for OriginToggled {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if event.is::<RoutineComplete<EndOfTurn>>() {
      self.log.borrow_mut().push("finished".to_string());
    }
  }

  fn is_active(&self, _ctx: &SystemContext) -> bool {
    self.active.get()
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
}

impl Harness {
  fn new(definition: RoutineDefinition) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut groups = Groups::new();
    groups.insert(ROOT_GROUP, SystemGroup::new());

    Self {
      base: AppState::default(),
      groups,
      routines: Routines::new(HashMap::from([(TypeId::of::<EndOfTurn>(), definition)])),
      tx,
      rx,
      tracers: TracerSenders::new(Vec::new()),
      log: Rc::default(),
    }
  }

  fn spawn(&mut self, group: &'static str, system: impl System + 'static) -> SystemHandle {
    let container = SystemContainer::new(system);
    let handle = SystemHandle::new(container.id(), group);
    self
      .groups
      .entry(group)
      .or_insert_with(SystemGroup::new)
      .insert(container);
    handle
  }

  fn step<const N: usize>(&mut self, group: &'static str, complete_now: bool) -> SystemHandle {
    let log = self.log.clone();
    self.spawn(group, Step::<N> { log, complete_now })
  }

  fn origin(&mut self, group: &'static str) -> SystemHandle {
    let log = self.log.clone();
    self.spawn(group, Origin { log })
  }

  fn ctx(&self, handle: SystemHandle) -> SystemContext<'_> {
    SystemContext::new(&self.base, handle, &self.groups, self.tx.clone())
  }

  /// Process routine messages the way the run loop does, until there are none left
  fn pump(&mut self) {
    while let Ok(message) = self.rx.try_recv() {
      match message {
        AppMessage::StartRoutine(start) => {
          self
            .routines
            .start(start, &self.groups, &self.base, &self.tx, &self.tracers)
        }
        AppMessage::CompleteRoutineStep(caller, type_id) => self.routines.complete_step(
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
      .get_mut(handle.parent_key)
      .unwrap()
      .remove(handle.id);
    self.changed(&[handle]);
  }

  fn start(&mut self, origin: SystemHandle) {
    self.ctx(origin).start_routine(EndOfTurn);
    self.pump();
  }

  fn complete(&mut self, participant: SystemHandle) {
    self.ctx(participant).complete_routine_step::<EndOfTurn>();
    self.pump();
  }

  fn set_group_active(&mut self, key: &'static str, active: bool) {
    let group = self.groups.get_mut(key).unwrap();
    if active {
      group.activate()
    } else {
      group.deactivate()
    }
    let handles = self.groups.handles_in(key);
    self.changed(&handles);
  }

  fn is_running(&self) -> bool {
    self.routines.is_running(TypeId::of::<EndOfTurn>())
  }

  fn log(&self) -> Vec<String> {
    self.log.borrow().clone()
  }
}

#[test]
fn runs_participants_in_order_then_signals_the_origin() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP);
  h.step::<2>(ROOT_GROUP, true);
  h.step::<1>(ROOT_GROUP, true);

  h.start(origin);

  assert_eq!(
    h.log(),
    ["step 1 in __root", "step 2 in __root", "finished"]
  );
  assert!(!h.is_running());
}

#[test]
fn waits_for_a_later_completion() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP);
  let first = h.step::<1>(ROOT_GROUP, false);
  h.step::<2>(ROOT_GROUP, true);

  h.start(origin);
  assert_eq!(h.log(), ["step 1 in __root"]);

  h.complete(first);
  assert_eq!(
    h.log(),
    ["step 1 in __root", "step 2 in __root", "finished"]
  );
}

#[test]
fn empty_and_undefined_routines_finish_immediately() {
  let mut h = Harness::new(routine![]);
  let origin = h.origin(ROOT_GROUP);
  h.start(origin);
  assert_eq!(h.log(), ["finished"]);

  let mut h = Harness::new(routine![]);
  h.routines = Routines::default();
  let origin = h.origin(ROOT_GROUP);
  h.start(origin);
  assert_eq!(h.log(), ["finished"]);
}

#[test]
fn a_second_start_is_ignored_while_running() {
  let mut h = Harness::new(routine![Step<1>]);
  let origin = h.origin(ROOT_GROUP);
  let step = h.step::<1>(ROOT_GROUP, false);

  h.start(origin);
  h.start(origin);
  assert_eq!(h.log(), ["step 1 in __root"]);

  h.complete(step);
  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}

#[test]
fn nearest_instance_wins() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  h.step::<1>(ROOT_GROUP, true);
  h.step::<1>("player", true);
  h.step::<2>(ROOT_GROUP, true);
  let origin = h.origin("player");

  h.start(origin);

  assert_eq!(
    h.log(),
    ["step 1 in player", "step 2 in __root", "finished"]
  );
}

#[test]
fn participants_out_of_reach_are_skipped() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  h.step::<1>("player", true);
  h.step::<2>(ROOT_GROUP, true);
  let origin = h.origin(ROOT_GROUP);

  h.start(origin);

  assert_eq!(h.log(), ["step 2 in __root", "finished"]);
}

#[test]
fn inactive_participants_are_skipped() {
  let mut h = Harness::new(routine![Toggled, Step<1>]);
  let origin = h.origin(ROOT_GROUP);
  h.spawn(
    ROOT_GROUP,
    Toggled {
      active: Rc::new(Cell::new(false)),
    },
  );
  h.step::<1>(ROOT_GROUP, true);

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}

#[test]
fn only_the_current_participant_can_complete() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP);
  let first = h.step::<1>(ROOT_GROUP, false);
  let second = h.step::<2>(ROOT_GROUP, false);

  h.start(origin);
  h.complete(second);
  h.complete(origin);
  assert_eq!(h.log(), ["step 1 in __root"]);

  // a duplicate completion from the first step doesn't skip the second
  h.ctx(first).complete_routine_step::<EndOfTurn>();
  h.ctx(first).complete_routine_step::<EndOfTurn>();
  h.pump();
  assert_eq!(h.log(), ["step 1 in __root", "step 2 in __root"]);
}

#[test]
fn a_participant_despawned_mid_step_counts_as_complete() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP);
  let first = h.step::<1>(ROOT_GROUP, false);
  h.step::<2>(ROOT_GROUP, true);

  h.start(origin);
  h.despawn(first);

  assert_eq!(
    h.log(),
    ["step 1 in __root", "step 2 in __root", "finished"]
  );
}

#[test]
fn a_participant_deactivated_mid_step_counts_as_complete() {
  let mut h = Harness::new(routine![Toggled, Step<1>]);
  let origin = h.origin(ROOT_GROUP);
  let active = Rc::new(Cell::new(true));
  h.spawn(
    ROOT_GROUP,
    Toggled {
      active: active.clone(),
    },
  );
  let toggled = h.groups[ROOT_GROUP].get_id::<Toggled>().unwrap();
  h.step::<1>(ROOT_GROUP, true);

  h.start(origin);
  assert!(h.log().is_empty());

  // nothing happens until the framework next checks the system
  active.set(false);
  assert!(h.log().is_empty());
  h.touch(SystemHandle::new(toggled, ROOT_GROUP));

  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}

#[test]
fn a_group_deactivated_and_reactivated_in_a_row_still_ends_the_step() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin("player");
  h.step::<1>("player", false);
  h.step::<2>(ROOT_GROUP, true);

  h.start(origin);
  // a poll in between would never see the group inactive
  h.set_group_active("player", false);
  h.set_group_active("player", true);

  assert_eq!(
    h.log(),
    ["step 1 in player", "step 2 in __root", "finished"]
  );
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
  for group in ["player 1", "player 2"] {
    let container = template.clone().to_system_container();
    h.groups
      .entry(group)
      .or_insert_with(SystemGroup::new)
      .insert(container);
  }
  let id = *h.groups["player 1"].ids()[0];
  h.step::<1>(ROOT_GROUP, true);
  let origin = h.origin("player 2");

  h.start(origin);
  h.set_group_active("player 1", false);
  h.complete(SystemHandle::new(id, "player 1"));
  assert!(h.log().is_empty());

  h.complete(SystemHandle::new(id, "player 2"));
  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}

#[test]
fn pauses_while_the_origin_is_inactive() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin("player");
  h.step::<1>("player", false);
  h.step::<2>(ROOT_GROUP, true);

  h.start(origin);
  // the step in progress counts as complete once its participant is inactive, but the routine goes no further
  h.set_group_active("player", false);
  assert_eq!(h.log(), ["step 1 in player"]);
  assert!(h.is_running());

  h.set_group_active("player", true);
  assert_eq!(
    h.log(),
    ["step 1 in player", "step 2 in __root", "finished"]
  );
}

#[test]
fn the_signal_waits_for_the_origin() {
  let mut h = Harness::new(routine![Step<1>]);
  let active = Rc::new(Cell::new(true));
  let origin = h.spawn(
    ROOT_GROUP,
    OriginToggled {
      log: h.log.clone(),
      active: active.clone(),
    },
  );
  let step = h.step::<1>(ROOT_GROUP, false);

  h.start(origin);
  active.set(false);
  h.complete(step);
  assert_eq!(h.log(), ["step 1 in __root"]);
  assert!(h.is_running());

  active.set(true);
  h.touch(origin);
  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}

#[test]
fn abandoned_when_the_origin_is_despawned() {
  let mut h = Harness::new(routine![Step<1>, Step<2>]);
  let origin = h.origin(ROOT_GROUP);
  let first = h.step::<1>(ROOT_GROUP, false);
  h.step::<2>(ROOT_GROUP, true);

  h.start(origin);
  h.despawn(origin);
  h.complete(first);

  assert_eq!(h.log(), ["step 1 in __root"]);
  assert!(!h.is_running());
}

#[test]
fn the_origin_gets_the_payload_back() {
  #[derive(Serialize, Event)]
  struct Drained {
    balls: u8,
  }

  struct Counter {
    log: Log,
  }

  impl System for Counter {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if let Some(done) = event.downcast_ref::<RoutineComplete<Drained>>() {
        self
          .log
          .borrow_mut()
          .push(format!("{} balls", done.payload.balls));
      }
    }
  }

  let mut h = Harness::new(routine![]);
  let log = h.log.clone();
  let origin = h.spawn(ROOT_GROUP, Counter { log });

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
  };
  step.on_routine(&EndOfTurn, &test_ctx.sys_ctx());
  assert!(test_ctx.routine_step_completed::<EndOfTurn>());

  test_ctx.sys_ctx().start_routine(EndOfTurn);
  let started = test_ctx.routines_started();
  assert_eq!(started.len(), 1);
  assert!(started[0].as_ref().is::<EndOfTurn>());
}

#[test]
fn participants_without_on_routine_complete() {
  let mut h = Harness::new(routine![Bystander, Step<1>]);
  let origin = h.origin(ROOT_GROUP);
  h.spawn(ROOT_GROUP, Bystander);
  h.step::<1>(ROOT_GROUP, true);

  h.start(origin);

  assert_eq!(h.log(), ["step 1 in __root", "finished"]);
}
