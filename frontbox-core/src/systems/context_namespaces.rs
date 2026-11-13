//! The namespaces on [`SystemContext`]: `ctx.systems`, `ctx.groups`, `ctx.cues`, `ctx.routines`, and `ctx.states`. Each
//! acts as the system the context was given to.

use std::any::{TypeId, type_name};
use std::cell::RefMut;

use crate::prelude::*;

/// `ctx.systems`: look up, spawn, and despawn systems
#[derive(Clone, Copy)]
pub struct ContextSystems<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextSystems<'a> {
  pub(crate) fn new(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// Lookup a system by type: the nearest instance matches firs
  pub fn get<T: System + 'static>(&self) -> Option<RefMut<'_, T>> {
    self.ctx.get::<T>(self.handle)
  }

  /// Lookup a system by type; panic if it does not exist or is already in use
  pub fn expect<T: System + 'static>(&self) -> RefMut<'_, T> {
    self.ctx.expect::<T>(self.handle)
  }

  /// Run `f` on the system `T`, looked up as [`get`](Self::get) does. Logs a warning and returns `None` if `T` isn't
  /// running, or an error if it's already in use.
  ///
  /// ```rust,ignore
  /// ctx.systems.with::<BallSaveSystem, _>(|ball_save| ball_save.request(ctx.id(), duration, ctx.into()));
  /// ```
  pub fn with<T: System + 'static, R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    match self.get::<T>() {
      Some(mut system) => Some(f(&mut system)),
      None => {
        // `get` has already logged an error if it's in use
        log::warn!("{} is not running or is in use.", type_name::<T>());
        None
      }
    }
  }

  /// Start up a new system in this system's group
  pub fn spawn(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.spawn_system(self.handle.group, system);
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.replace_self(self.handle, system);
  }

  pub fn despawn_self(&self) {
    self.ctx.despawn_self(self.handle);
  }
}

/// `ctx.groups`: spawn and manage [system groups](mod@crate::systems::system_group)
#[derive(Clone, Copy)]
pub struct ContextGroups<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextGroups<'a> {
  pub(crate) fn new(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// Spawn a group of systems, owned by this system. The returned handle is what activating, deactivating, and
  /// despawning the group take. The group is despawned along with this system.
  pub fn spawn(
    &self,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) -> GroupHandle {
    self
      .ctx
      .spawn_system_group(self.handle, group_name, systems, active)
  }

  pub fn despawn(&self, group: GroupHandle) {
    self.ctx.despawn_system_group(group);
  }

  pub fn activate(&self, group: &GroupHandle) {
    self.ctx.activate_system_group(group);
  }

  pub fn deactivate(&self, group: &GroupHandle) {
    self.ctx.deactivate_system_group(group);
  }
}

/// `ctx.cues`: schedule signals back to this system. See [cues](mod@crate::systems::cue)
#[derive(Clone, Copy)]
pub struct ContextCues<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextCues<'a> {
  pub(crate) fn new(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// Start a cue that sends `signal` to this system as `cue` says. Returns its id, for [`cancel`](Self::cancel).
  pub fn start(&self, signal: impl Event + 'static, cue: Cue) -> u64 {
    self.ctx.cue(self.handle, signal, cue)
  }

  /// Start a cue that sends multiple `signals`, in turn, cycling through each
  pub fn start_multi(&self, signals: Vec<Box<dyn Event>>, cue: Cue) -> u64 {
    self.ctx.cue_cycling(self.handle, signals, cue)
  }

  /// Cancel a cue this system started. Takes an id or an `Option` of one, and does nothing for `None`.
  pub fn cancel(&self, cue_id: impl Into<Option<u64>>) {
    if let Some(cue_id) = cue_id.into() {
      self.ctx.cancel_cue(self.handle, cue_id);
    }
  }
}

/// `ctx.routines`: start and take part in [routines](crate::systems::routine)
#[derive(Clone, Copy)]
pub struct ContextRoutines<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextRoutines<'a> {
  // not `new`, which is taken by `ctx.routines.new(routine)`
  pub(crate) fn for_system(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// Start a routine. [`RoutineStarted<E>`] is broadcast before the first participant is handed the routine, and
  /// [`RoutineCompleted<E>`] once the last one is done. Participants are looked up from this system's group. Returns
  /// its id, for [`cancel`](Self::cancel). Short for `ctx.routines.new(routine).start()`.
  pub fn start<E: Event + Clone>(&self, routine: E) -> u64 {
    self.new(routine).start()
  }

  /// A routine to start, for setting how it runs (e.g. [`RoutineBuilder::group`], [`RoutineBuilder::interval`])
  /// before calling [`RoutineBuilder::start`]
  pub fn new<E: Event + Clone>(&self, routine: E) -> RoutineBuilder<'a> {
    RoutineBuilder::new(self.handle, self.ctx, routine)
  }

  /// Stop a routine this system started, wherever it's at. The participant whose step is in progress isn't told, and
  /// no one after it takes a step. [`RoutineCompleted<E>`] is broadcast with `RoutineCompletion::Cancelled`. Does
  /// nothing once the routine has completed, or if another system started it. Takes an id or an `Option` of one, and
  /// does nothing for `None`.
  pub fn cancel(&self, routine_id: impl Into<Option<u64>>) {
    if let Some(routine_id) = routine_id.into() {
      self.ctx.cancel_routine(self.handle, routine_id);
    }
  }

  /// End this system's step in the routine for `E`, handing it to the next participant
  pub fn complete_step<E: Event>(&self) {
    self.complete_step_by_type_id(TypeId::of::<E>());
  }

  pub(crate) fn complete_step_by_type_id(&self, type_id: TypeId) {
    self.ctx.complete_routine_step(self.handle, type_id);
  }

  /// End the routine for `E` with this system's step, so the participants after it don't take one. For a participant
  /// that settles what the routine is deciding (e.g. a ball save on a ball-draining routine). `RoutineCompleted<E>` is
  /// broadcast with `RoutineCompletion::Halted`, so the origin can tell. Only the current participant can halt it.
  pub fn halt<E: Event>(&self) {
    self.ctx.halt_routine(self.handle, TypeId::of::<E>());
  }

  pub(crate) fn report_activity_changed(&self) {
    self.ctx.report_activity_changed(self.handle);
  }
}

/// `ctx.states`: read and set [states](crate::systems::state)
#[derive(Clone, Copy)]
pub struct ContextStates<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextStates<'a> {
  pub(crate) fn new(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// The current value of the state `S`: as set in this system's group, then the root. The default if neither has set
  /// it.
  pub fn get<S: State>(&self) -> S {
    self.ctx.state::<S>(self.handle)
  }

  /// Whether the state `S` currently has the value `value`
  pub fn is<S: State>(&self, value: S) -> bool {
    self.get::<S>() == value
  }

  /// Whether the state `S` currently has any of the values in `values`
  pub fn is_any<S: State>(&self, values: &[S]) -> bool {
    values.contains(&self.get::<S>())
  }

  /// Set the state `S` in this system's group. The first system to set a state owns it, and only the owner can set it
  /// again. Takes effect in queue order, so this system reads the old value until then; [`StateChanged<S>`] is
  /// broadcast once it's set, unless the value didn't change.
  pub fn set<S: State>(&self, value: S) {
    self.ctx.set_state(self.handle, value);
  }
}
