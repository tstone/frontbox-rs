use std::any::{TypeId, type_name};
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::app::app_message::EventBox;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

/// # Context
/// 
/// Each handler receives a reference to `Context`. As this guide has shown, it's through Context that access several features is provided, including:
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
pub struct Context<'a> {
  base: &'a ContextBase,
  handle: SystemHandle,
  /// Access to sibling and root systems
  pub systems: SystemsContext<'a>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
}

impl<'a> Context<'a> {
  pub fn new(
    base: &'a ContextBase,
    handle: SystemHandle,
    groups: &'a Groups,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      base,
      systems: SystemsContext {
        groups,
        parent_key: handle.parent_key,
      },
      handle,
      app_sender,
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
    log::debug!(
      "📨 Emitting event {} {:?}",
      type_name::<E>(),
      event.type_id()
    );
    self
      .app_sender
      .send(AppMessage::EmitEvent(EventBox::new(event)))
      .ok();
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  /// See [event interrupts](crate::systems::event_interrupts)
  pub fn register_interrupt<E: Event + 'static>(&self, priority: u16) {
    log::debug!(
      "Registering interrupt for {} by {}",
      type_name::<E>(),
      self.handle.id,
    );
    self
      .app_sender
      .send(AppMessage::RegisterInterrupt(
        self.handle,
        TypeId::of::<E>(),
        priority,
      ))
      .ok();
  }

  pub fn unregister_interrupt<E: Event + 'static>(&self) {
    log::debug!(
      "Unregistering interrupt for {} by {}",
      type_name::<E>(),
      self.handle.id,
    );
    self
      .app_sender
      .send(AppMessage::UnregisterInterrupt(
        self.handle.id,
        TypeId::of::<E>(),
      ))
      .ok();
  }

  // --- System management ---

  /// Start up a new system
  pub fn spawn_system(&self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self.app_sender.send(AppMessage::SpawnSystem(
      self.handle.parent_key,
      system.into(),
    ));
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(self.handle, system.into()));
  }

  pub fn despawn_self(&self) {
    let _ = self.app_sender.send(AppMessage::DespawnSystem(self.handle));
  }

  pub fn spawn_system_group(
    &self,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystemGroup(group_name, systems, active));
  }

  pub fn despawn_system_group(&self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystemGroup(group_name));
  }

  pub fn activate_system_group(&self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::ActivateSystemGroup(group_name));
  }

  pub fn deactivate_system_group(&self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::DeactivateSystemGroup(group_name));
  }

  // --- Cues ---

  pub fn cue(&self, signal: impl Event + 'static, cue: Cue) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(
        self.handle,
        cue_id,
        cue,
        vec![Box::new(signal)],
      ))
      .ok();
    cue_id
  }

  pub fn cue_cycling(&self, signals: Vec<Box<dyn Event>>, cue: Cue) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(self.handle, cue_id, cue, signals))
      .ok();
    cue_id
  }

  pub fn cue_timeline(&self, timeline: CueTimeline) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCueTimeline(self.handle, cue_id, timeline))
      .ok();
    cue_id
  }

  pub fn cancel_cue(&self, cue_id: u64) {
    let _ = self
      .app_sender
      .send(AppMessage::CancelCue(self.handle, cue_id));
  }

  pub fn clone_for_system(&self, handle: SystemHandle) -> Context<'a> {
    Context {
      base: self.base,
      handle,
      systems: self.systems.clone(),
      app_sender: self.app_sender.clone(),
    }
  }
}

impl Deref for Context<'_> {
  type Target = ContextBase;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}
