use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

#[derive(Debug)]
pub struct Context<'a> {
  pub(crate) store: &'a mut Store,
  pub(crate) system_id: u64,
  pub(crate) app_sender: mpsc::UnboundedSender<AppMessage>,
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

  pub fn emit<E: Event>(&mut self, event: E) {
    self
      .app_sender
      .send(AppMessage::EmitEvent(Box::new(event)))
      .ok();
  }

  // -- Commands --

  pub fn command<C: Command>(&mut self, cmd: C) {
    self
      .app_sender
      .send(AppMessage::ExecuteCommand(self.system_id, Box::new(cmd)))
      .ok();
  }

  /// Register a command handler
  pub fn register_command<C: Command + 'static>(&mut self) {
    self
      .app_sender
      .send(AppMessage::RegisterCommand(
        self.system_id,
        TypeId::of::<C>(),
      ))
      .ok();
  }

  pub fn unregister_command<C: Command + 'static>(&mut self) {
    self
      .app_sender
      .send(AppMessage::UnregisterCommand(
        self.system_id,
        TypeId::of::<C>(),
      ))
      .ok();
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  pub fn register_interrupt<E: Event + 'static>(&mut self, priority: u16) {
    self
      .app_sender
      .send(AppMessage::RegisterInterrupt(
        self.system_id,
        TypeId::of::<E>(),
        priority,
      ))
      .ok();
  }

  pub fn unregister_interrupt<E: Event + 'static>(&mut self) {
    self
      .app_sender
      .send(AppMessage::UnregisterInterrupt(
        self.system_id,
        TypeId::of::<E>(),
      ))
      .ok();
  }

  // --- System management ---

  pub fn spawn_system(&mut self, system: impl SpawnableSystem + 'static) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystem(self.system_id, Box::new(system)));
  }

  pub fn replace_system(&mut self, system: impl SpawnableSystem + 'static) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(self.system_id, Box::new(system)));
  }

  pub fn despawn_system(&mut self) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystem(self.system_id));
  }

  pub fn spawn_system_group(
    &mut self,
    group_name: &'static str,
    systems: Vec<Box<dyn ChildSystem>>,
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

  // --- Timer ---

  pub fn set_timer(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    let _ = self.app_sender.send(AppMessage::SetTimer(
      self.system_id,
      timer_name,
      duration,
      mode,
    ));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    let _ = self
      .app_sender
      .send(AppMessage::ClearTimer(self.system_id, timer_name));
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
