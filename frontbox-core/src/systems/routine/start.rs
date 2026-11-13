use std::any::{TypeId, type_name};
use std::fmt::Debug;

use crate::app::event_box::EventBox;
use crate::prelude::*;

/// A routine to start, and how to run it, from `ctx.routines.new`. Nothing happens until [`start`](Self::start).
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # #[derive(Serialize, Event, Clone)]
/// # struct Searching;
/// # fn example(ctx: &SystemContext, player: &GroupHandle) {
/// let routine_id = ctx.routines.new(Searching).group(player).interval(Duration::from_millis(250)).start();
/// # }
/// ```
#[must_use = "a routine doesn't run until `start` is called"]
pub struct RoutineBuilder<'a> {
  pub(crate) origin: SystemHandle,
  pub(crate) ctx: ServiceContext<'a>,
  pub(crate) type_id: TypeId,
  pub(crate) name: &'static str,
  pub(crate) routine: Box<dyn Event>,
  pub(crate) into_started: fn(&dyn Event) -> EventBox,
  pub(crate) into_completed: fn(Box<dyn Event>, RoutineCompletion) -> EventBox,
  pub(crate) scope: Option<GroupId>,
  pub(crate) interval: Option<Duration>,
}

impl<'a> RoutineBuilder<'a> {
  pub(crate) fn new<E: Event + Clone>(origin: SystemHandle, ctx: ServiceContext<'a>, routine: E) -> Self {
    Self {
      origin,
      ctx,
      type_id: TypeId::of::<E>(),
      name: type_name::<E>(),
      routine: Box::new(routine),
      into_started: RoutineStarted::<E>::from_routine,
      into_completed: RoutineCompleted::<E>::from_routine,
      scope: None,
      interval: None,
    }
  }

  /// Find participants starting in `group` rather than the origin's own group
  pub fn group(mut self, group: &GroupHandle) -> Self {
    self.scope = Some(group.id());
    self
  }

  /// Wait this long after a participant completes its step before handing the routine to the next one
  pub fn interval(mut self, interval: Duration) -> Self {
    self.interval = Some(interval);
    self
  }

  /// Start the routine. Returns its id, for `ctx.routines.cancel`.
  pub fn start(self) -> u64 {
    let ctx = self.ctx;
    ctx.start_routine(self)
  }

  pub(crate) fn into_start(self, id: u64) -> RoutineStart {
    RoutineStart {
      id,
      origin: self.origin,
      scope: self.scope.unwrap_or(self.origin.group),
      type_id: self.type_id,
      name: self.name,
      routine: self.routine,
      into_started: self.into_started,
      into_completed: self.into_completed,
      interval: self.interval,
    }
  }
}

/// A request to start a routine, sent from a context to the run loop
pub struct RoutineStart {
  /// Returned by `ctx.routines.start`, for the origin to cancel it with
  pub(crate) id: u64,
  pub(crate) origin: SystemHandle,
  /// The group participants are looked up from: the nearest instance, in the group itself, then the root
  pub(crate) scope: GroupId,
  pub(crate) type_id: TypeId,
  pub(crate) name: &'static str,
  pub(crate) routine: Box<dyn Event>,
  /// Copies the routine into its `RoutineStarted`. Chosen here, while its type is still known
  pub(crate) into_started: fn(&dyn Event) -> EventBox,
  /// Wraps the routine in its `RoutineCompleted`
  pub(crate) into_completed: fn(Box<dyn Event>, RoutineCompletion) -> EventBox,
  /// Wait between one participant completing its step and the next being handed the routine
  pub(crate) interval: Option<Duration>,
}

impl Debug for RoutineStart {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} from {}", self.name, self.origin.id)
  }
}
