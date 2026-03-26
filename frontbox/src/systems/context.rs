use std::any::{TypeId, type_name};
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub struct Context<'a> {
  base: &'a ContextBase,
  system_id: u64,
  app_sender: mpsc::UnboundedSender<AppMessage>,
}

impl<'a> Context<'a> {
  pub fn new(
    base: &'a ContextBase,
    system_id: u64,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      base,
      system_id,
      app_sender,
    }
  }

  pub fn emit<E: Event>(&self, event: E) {
    log::debug!("📨 Emitting event {}", type_name::<E>());
    self
      .app_sender
      .send(AppMessage::EmitEvent(Box::new(event)))
      .ok();
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  pub fn register_interrupt<E: Event + 'static>(&self, priority: u16) {
    self
      .app_sender
      .send(AppMessage::RegisterInterrupt(
        self.system_id,
        TypeId::of::<E>(),
        priority,
      ))
      .ok();
  }

  pub fn unregister_interrupt<E: Event + 'static>(&self) {
    self
      .app_sender
      .send(AppMessage::UnregisterInterrupt(
        self.system_id,
        TypeId::of::<E>(),
      ))
      .ok();
  }

  // --- System management ---

  /// Start up a new system
  pub fn spawn_system(&self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystem(self.system_id, system.into()));
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(self.system_id, system.into()));
  }

  pub fn despawn_self(&self) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystem(self.system_id));
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
        self.system_id,
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
      .send(AppMessage::CreateCue(self.system_id, cue_id, cue, signals))
      .ok();
    cue_id
  }

  pub fn cancel_cue(&self, cue_id: u64) {
    let _ = self
      .app_sender
      .send(AppMessage::CancelCue(self.system_id, cue_id));
  }

  pub fn clone_for_system(&self, system_id: u64) -> Context<'a> {
    Context {
      base: self.base,
      system_id,
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
