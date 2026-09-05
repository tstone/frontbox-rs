use std::cell::RefMut;
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;

/// # SystemContext
///
/// Each handler receives a reference to `SystemContext` which has both the `Context` and a reference to the `SystemHandle` of the current system. As this guide has shown, it's through Context that access several features is provided, including:
///
/// - Register cues and interrupts
/// - Emit events
/// - Access hardware configuration and state
///
/// ```rust
/// ctx.switches
/// ctx.drivers
/// ctx.io_network
/// ctx.exp_network
/// ```
///
/// **Important**: SystemContext is specific to the system it was given, and should not be shared to other systems.
pub struct SystemContext<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
}

impl<'a> SystemContext<'a> {
  pub fn new(
    base: &'a BootSnapshot,
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
    self.ctx.emit(event);
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  /// See [event interrupts](crate::systems::event_interrupts)
  pub fn register_interrupt<E: Event + 'static>(&self, priority: u16) {
    self.ctx.register_interrupt::<E>(self.handle, priority);
  }

  /// Remove a previously registered interrupt handler
  pub fn unregister_interrupt<E: Event + 'static>(&self) {
    self.ctx.unregister_interrupt::<E>(self.handle);
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
    self.ctx.spawn_system(self.handle.parent_key, system);
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    self.ctx.replace_self(self.handle, system);
  }

  pub fn despawn_self(&self) {
    self.ctx.despawn_self(self.handle);
  }

  pub fn spawn_system_group(
    &self,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) {
    self.ctx.spawn_system_group(group_name, systems, active);
  }

  pub fn despawn_system_group(&self, group_name: &'static str) {
    self.ctx.despawn_system_group(group_name);
  }

  pub fn activate_system_group(&self, group_name: &'static str) {
    self.ctx.activate_system_group(group_name);
  }

  pub fn deactivate_system_group(&self, group_name: &'static str) {
    self.ctx.deactivate_system_group(group_name);
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
  type Target = BootSnapshot;

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
