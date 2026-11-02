use std::any::TypeId;
use std::cell::RefMut;
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;

/// # SystemContext
///
/// Each handler receives a reference to `SystemContext` which has both the `Context` and a reference to the `SystemHandle` of the current system. As this guide has shown, it's through Context that access several features is provided, including:
///
/// - Schedule cues
/// - Emit events
/// - Access hardware configuration and state
///
/// ```rust
/// # use frontbox::prelude::*;
/// # fn example(ctx: &SystemContext) {
/// let switches = &ctx.switches;
/// let drivers = &ctx.drivers;
/// let io_network = &ctx.io_network;
/// let exp_network = &ctx.exp_network;
/// # }
/// ```
///
/// **Important**: SystemContext is specific to the system it was given, and should not be shared to other systems.
pub struct SystemContext<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> SystemContext<'a> {
  pub fn new(
    base: &'a AppState,
    handle: SystemHandle,
    groups: &'a Groups,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      handle,
      ctx: ServiceContext::new(base, groups, app_sender),
    }
  }

  /// Each system is automatically assigned an internal ID. This returns the ID for the current system.
  pub fn current_system_id(&self) -> u64 {
    self.handle.id
  }

  /// The complete handle (parent + ID) for the current system
  pub fn current_handle(&self) -> &SystemHandle {
    &self.handle
  }

  /// Dispatch an event to all other active systems
  pub fn emit<E: Event>(&self, event: E) {
    self.ctx.emit(self.current_system_id(), event);
  }
  // -- Systems access --

  /// Lookup a system by type
  pub fn get<T: System + 'static>(&self) -> Option<RefMut<'_, T>> {
    self.ctx.get::<T>(self.handle)
  }

  /// Lookup a system by type; panic if does not exist
  pub fn expect<T: System + 'static>(&self) -> RefMut<'_, T> {
    self.ctx.expect::<T>(self.handle)
  }

  /// Start up a new system
  pub fn spawn_system(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.spawn_system(self.handle.group, system);
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.replace_self(self.handle, system);
  }

  pub fn despawn_self(&self) {
    self.ctx.despawn_self(self.handle);
  }

  /// Spawn a group of systems, owned by this system. The returned handle is what activating, deactivating, and
  /// despawning the group take. The group is despawned along with this system. See
  /// [system groups](mod@crate::systems::system_group).
  pub fn spawn_system_group(
    &self,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) -> GroupHandle {
    self
      .ctx
      .spawn_system_group(self.handle, group_name, systems, active)
  }

  pub fn despawn_system_group(&self, group: GroupHandle) {
    self.ctx.despawn_system_group(group);
  }

  pub fn activate_system_group(&self, group: &GroupHandle) {
    self.ctx.activate_system_group(group);
  }

  pub fn deactivate_system_group(&self, group: &GroupHandle) {
    self.ctx.deactivate_system_group(group);
  }

  pub fn cue(&self, signal: impl Event + 'static, cue: Cue) -> u64 {
    self.ctx.cue(self.handle, signal, cue)
  }

  pub fn cue_cycling(&self, signals: Vec<Box<dyn Event>>, cue: Cue) -> u64 {
    self.ctx.cue_cycling(self.handle, signals, cue)
  }

  pub fn cue_timeline(&self, timeline: CueTimeline) -> u64 {
    self.ctx.cue_timeline(self.handle, timeline)
  }

  pub fn cancel_cue(&self, cue_id: u64) {
    self.ctx.cancel_cue(self.handle, cue_id);
  }

  // -- Routines --

  /// Start the routine for `E`. [`RoutineStarted<E>`] is broadcast before the first participant is
  /// handed the routine, and [`RoutineCompleted<E>`] once the last one is done. See [routines](crate::systems::routine)
  pub fn start_routine<E: Event + Clone>(&self, routine: E) {
    self
      .ctx
      .start_routine(self.handle, self.handle.group, routine);
  }

  /// Start the routine for `E`, with its participants looked up from `group` rather than this system's own group: the
  /// nearest instance of each, in `group` itself, then the root. This system is still the origin, so it's the one whose
  /// activity pauses the routine. Taking the handle means only the group's owner (or whoever it shares the handle with)
  /// can start a routine in it.
  pub fn start_routine_in<E: Event + Clone>(&self, group: &GroupHandle, routine: E) {
    self.ctx.start_routine(self.handle, group.id(), routine);
  }

  /// End this system's step in the routine for `E`, handing it to the next participant
  pub fn complete_routine_step<E: Event>(&self) {
    self.complete_routine_step_by_type_id(TypeId::of::<E>());
  }

  pub(crate) fn complete_routine_step_by_type_id(&self, type_id: TypeId) {
    self.ctx.complete_routine_step(self.handle, type_id);
  }

  /// End the routine for `E` with this system's step, so the participants after it don't take one. For a participant
  /// that settles what the routine is deciding (e.g. a ball save on a ball-draining routine). `RoutineCompleted<E>` is
  /// broadcast with `RoutineCompletion::Halted`, so the origin can tell. Only the current participant can halt it.
  pub fn halt_routine<E: Event>(&self) {
    self.ctx.halt_routine(self.handle, TypeId::of::<E>());
  }

  pub(crate) fn report_activity_changed(&self) {
    self.ctx.report_activity_changed(self.handle);
  }

  pub fn clone_for_system(&self, handle: SystemHandle) -> SystemContext<'a> {
    SystemContext {
      handle,
      ctx: self.ctx.clone(),
    }
  }

  /// Unregisters drivers, clears the EXP network, and halts the framework
  pub fn shutdown(&self, scope: ShutdownScope) {
    self.ctx.shutdown(scope);
  }
}

impl Deref for SystemContext<'_> {
  type Target = AppState;

  fn deref(&self) -> &Self::Target {
    self.ctx.base
  }
}

impl<'a> Into<ServiceContext<'a>> for SystemContext<'a> {
  fn into(self) -> ServiceContext<'a> {
    self.ctx
  }
}

impl<'a, 'b> Into<&'b ServiceContext<'a>> for &'b SystemContext<'a> {
  fn into(self) -> &'b ServiceContext<'a> {
    &self.ctx
  }
}
