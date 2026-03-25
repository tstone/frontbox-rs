use std::any::{TypeId, type_name};
use std::ops::{Deref, DerefMut};

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

#[derive(Debug)]
pub struct Context<'a> {
  store: &'a mut Store,
  system_id: u64,
  app_sender: mpsc::UnboundedSender<AppMessage>,
}

impl<'a> Context<'a> {
  pub fn new(
    store: &'a mut Store,
    system_id: u64,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      store,
      system_id,
      app_sender,
    }
  }

  /// Check if a value of type T exists in the store and is equal to the given value
  pub fn is<T: StorableType + PartialEq>(&self, value: T) -> bool {
    if let Some(stored) = self.get::<T>() {
      return *stored == value;
    }
    false
  }

  pub fn emit<E: Signal>(&mut self, event: E) {
    log::debug!("📨 Emitting event {}", type_name::<E>());
    self
      .app_sender
      .send(AppMessage::EmitEvent(Box::new(event)))
      .ok();
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  pub fn register_interrupt<E: Signal + 'static>(&mut self, priority: u16) {
    self
      .app_sender
      .send(AppMessage::RegisterInterrupt(
        self.system_id,
        TypeId::of::<E>(),
        priority,
      ))
      .ok();
  }

  pub fn unregister_interrupt<E: Signal + 'static>(&mut self) {
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
  pub fn spawn_system(&mut self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystem(self.system_id, system.into()));
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&mut self, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(self.system_id, system.into()));
  }

  pub fn despawn_self(&mut self) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystem(self.system_id));
  }

  pub fn spawn_system_group(
    &mut self,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystemGroup(group_name, systems, active));
  }

  pub fn despawn_system_group(&mut self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystemGroup(group_name));
  }

  pub fn activate_system_group(&mut self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::ActivateSystemGroup(group_name));
  }

  pub fn deactivate_system_group(&mut self, group_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::DeactivateSystemGroup(group_name));
  }

  // --- Cues ---

  pub fn cue(&mut self, signal: impl Signal + 'static, cue: Cue) -> u64 {
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

  pub fn cue_cycling(&mut self, signals: Vec<Box<dyn Signal>>, cue: Cue) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(self.system_id, cue_id, cue, signals))
      .ok();
    cue_id
  }

  pub fn cancel_cue(&mut self, cue_id: u64) {
    let _ = self
      .app_sender
      .send(AppMessage::CancelCue(self.system_id, cue_id));
  }

  pub fn clone_for_system(&mut self, system_id: u64) -> Context<'_> {
    Context {
      store: self.store,
      system_id,
      app_sender: self.app_sender.clone(),
    }
  }
}

impl Deref for Context<'_> {
  type Target = Store;

  fn deref(&self) -> &Self::Target {
    self.store
  }
}

impl DerefMut for Context<'_> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.store
  }
}
