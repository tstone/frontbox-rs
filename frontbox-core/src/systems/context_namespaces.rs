//! The namespaces on [`SystemContext`]: `ctx.systems`, `ctx.timers`, `ctx.routines`, and `ctx.states`.
//! Each acts as the system the context was given to.

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

  /// Lookup a system by type: among this system's children, then its siblings, then its parent's, and so on up to the
  /// top level. The nearest instance matches first.
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
        log::warn!(target: "frontbox::systems", "{} is not running or is in use.", type_name::<T>());
        None
      }
    }
  }

  /// Lookup a system by a trait it provides (see [`System::provides`]): the nearest one matches first
  ///
  /// ```rust,ignore
  /// if let Some(mut balls) = ctx.systems.get_dyn::<dyn BallManager>() { /* ... */ }
  /// ```
  pub fn get_dyn<I: ?Sized + 'static>(&self) -> Option<RefMut<'_, I>> {
    self.ctx.get_dyn::<I>(self.handle)
  }

  /// Lookup a system by a trait it provides; panic if none does or it's already in use
  pub fn expect_dyn<I: ?Sized + 'static>(&self) -> RefMut<'_, I> {
    self.ctx.expect_dyn::<I>(self.handle)
  }

  /// Run `f` on the system providing the trait `I`, looked up as [`get_dyn`](Self::get_dyn) does. Logs a warning and
  /// returns `None` if none is running, or an error if it's already in use.
  pub fn with_dyn<I: ?Sized + 'static, R>(&self, f: impl FnOnce(&mut I) -> R) -> Option<R> {
    match self.get_dyn::<I>() {
      Some(mut system) => Some(f(&mut system)),
      None => {
        // `get_dyn` has already logged an error if it's in use
        log::warn!(target: "frontbox::systems", "No system providing {} is running, or it is in use.", type_name::<I>());
        None
      }
    }
  }

  /// Lookup a system by type among the children of the system with id `parent`, such as one player's [`Group`]. For
  /// a system that isn't a sibling or ancestor of the caller's, which a plain [`get`](Self::get) doesn't reach.
  pub fn get_in<T: System + 'static>(&self, parent: u64) -> Option<RefMut<'_, T>> {
    self.ctx.get_in::<T>(parent, self.handle)
  }

  /// The name of the running system with this id, such as `frontbox_pinball::scoop::ScoopSystem<lotko::Lower>`.
  /// Readable while that system is in use.
  pub fn name_of(&self, id: u64) -> Option<&'static str> {
    self.ctx.name_of(id)
  }

  /// Start up a new system as a child of this one. Returns its id, which [`activate`](Self::activate),
  /// [`deactivate`](Self::deactivate), and [`despawn`](Self::despawn) take. The child is despawned along with this
  /// system, and is only active while this system is.
  pub fn spawn(&self, system: impl Into<SpawnableSystemContainer>) -> u64 {
    self.ctx.spawn_child(self.handle, system)
  }

  /// Let a child of this system run again, after [`deactivate`](Self::deactivate). Its own children follow. Does
  /// nothing for a system that isn't this one's child, logging a warning.
  pub fn activate(&self, child: u64) {
    self.ctx.set_child_active(self.handle, child, true);
  }

  /// Stop a child of this system, and its children, until [`activate`](Self::activate)d again. Does nothing for a
  /// system that isn't this one's child, logging a warning.
  pub fn deactivate(&self, child: u64) {
    self.ctx.set_child_active(self.handle, child, false);
  }

  /// Despawn a child of this system, and its children. Does nothing for a system that isn't this one's child, logging a
  /// warning.
  pub fn despawn(&self, child: u64) {
    self.ctx.despawn_child(self.handle, child);
  }

  /// Despawn self, and its children, and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.replace_self(self.handle, system);
  }

  /// Despawn self, and its children
  pub fn despawn_self(&self) {
    self.ctx.despawn_self(self.handle);
  }
}

/// `ctx.timers`: schedule signals back to this system. See [timers](mod@crate::systems::timer)
#[derive(Clone, Copy)]
pub struct ContextTimers<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> ContextTimers<'a> {
  pub(crate) fn new(handle: SystemHandle, ctx: ServiceContext<'a>) -> Self {
    Self { handle, ctx }
  }

  /// Start a timer that sends `signal` to this system as `timer` says. Returns its id, for [`cancel`](Self::cancel).
  pub fn start(&self, signal: impl Event + 'static, timer: Timer) -> u64 {
    self.ctx.timer(self.handle, signal, timer)
  }

  /// Start a timer that sends multiple `signals`, in turn, cycling through each
  pub fn start_multi(&self, signals: Vec<Box<dyn Event>>, timer: Timer) -> u64 {
    self.ctx.timer_cycling(self.handle, signals, timer)
  }

  /// Cancel a timer this system started. Takes an id or an `Option` of one, and does nothing for `None`.
  pub fn cancel(&self, timer_id: impl Into<Option<u64>>) {
    if let Some(timer_id) = timer_id.into() {
      self.ctx.cancel_timer(self.handle, timer_id);
    }
  }

  /// Pause a timer this system started: time stops counting toward it until it's [resumed](Self::resume). Takes an id
  /// or an `Option` of one, and does nothing for `None` or a timer that's already paused, completed, or canceled.
  pub fn pause(&self, timer_id: impl Into<Option<u64>>) {
    if let Some(timer_id) = timer_id.into() {
      self.ctx.set_timer_paused(self.handle, timer_id, true);
    }
  }

  /// Resume a timer [paused](Self::pause) by this system, picking up where it left off. Takes an id or an `Option` of
  /// one, and does nothing for `None` or a timer that isn't paused.
  pub fn resume(&self, timer_id: impl Into<Option<u64>>) {
    if let Some(timer_id) = timer_id.into() {
      self.ctx.set_timer_paused(self.handle, timer_id, false);
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
  /// [`RoutineCompleted<E>`] once the last one is done. Every active system of each listed type takes a step, wherever
  /// it is. Returns its id, for [`cancel`](Self::cancel). Short for `ctx.routines.new(routine).start()`.
  pub fn start<E: Event + Clone>(&self, routine: E) -> u64 {
    self.new(routine).start()
  }

  /// A routine to start, for setting how it runs (e.g. [`RoutineBuilder::interval`])
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

  /// The current value of the state `S`, or its default if it hasn't been set
  pub fn get<S: State>(&self) -> S {
    self.ctx.state::<S>()
  }

  /// Whether the state `S` currently has the value `value`
  pub fn is<S: State>(&self, value: S) -> bool {
    self.get::<S>() == value
  }

  /// Whether the state `S` currently has any of the values in `values`
  pub fn is_any<S: State>(&self, values: &[S]) -> bool {
    values.contains(&self.get::<S>())
  }

  /// Set the state `S`, for the whole machine. The first system to set a state owns it, and only the owner can set it
  /// again. Takes effect in queue order, so this system reads the old value until then; [`StateChanged<S>`] is
  /// broadcast once it's set, unless the value didn't change.
  pub fn set<S: State>(&self, value: S) {
    self.ctx.set_state(self.handle, value);
  }
}
