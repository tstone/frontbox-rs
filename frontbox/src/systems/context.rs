use std::any::{Any, TypeId};
use std::ops::{Deref, DerefMut};

use serde_json::Value;
use tokio::sync::mpsc;

use crate::prelude::*;
use crate::systems::SystemMessage;

#[derive(Debug)]
pub struct Context<'a> {
  store: &'a mut Store,
  system_id: u64,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  system_sender: mpsc::UnboundedSender<SystemMessage>,
}

impl<'a> Context<'a> {
  pub fn new(
    store: &'a mut Store,
    system_id: u64,
    app_sender: mpsc::UnboundedSender<AppMessage>,
    system_sender: mpsc::UnboundedSender<SystemMessage>,
  ) -> Self {
    Self {
      store,
      system_id,
      app_sender,
      system_sender,
    }
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
  pub fn register_command<C: Command + 'static>(
    &mut self,
    runner: impl for<'ctx> Fn(&C, &mut Context<'ctx>) + Send + Sync + 'static,
  ) {
    self
      .app_sender
      .send(AppMessage::RegisterCommand(
        TypeId::of::<C>(),
        Box::new(move |cmd: &dyn Any, ctx: &mut Context| {
          if let Some(cmd) = cmd.downcast_ref::<C>() {
            runner(cmd, ctx);
          }
        }),
      ))
      .ok();
  }

  // --- System management ---

  pub fn spawn_system(&mut self, system: impl System + 'static) {
    let _ = self
      .system_sender
      .send(SystemMessage::SpawnSystem(Box::new(system)));
  }

  pub fn replace_system(&mut self, system: impl System + 'static) {
    let _ = self.system_sender.send(SystemMessage::ReplaceSystem(
      self.system_id,
      Box::new(system),
    ));
  }

  pub fn despawn_system(&mut self) {
    let _ = self
      .system_sender
      .send(SystemMessage::DespawnSystem(self.system_id));
  }

  // --- Timer ---

  pub fn set_timer(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    let _ = self.system_sender.send(SystemMessage::SetTimer(
      self.system_id,
      timer_name,
      duration,
      mode,
    ));
  }

  pub fn clear_timer(&mut self, timer_name: &'static str) {
    let _ = self
      .system_sender
      .send(SystemMessage::ClearTimer(self.system_id, timer_name));
  }

  pub fn clone_for_system(&mut self, system_id: u64) -> Context {
    Context {
      store: self.store,
      system_id,
      app_sender: self.app_sender.clone(),
      system_sender: self.system_sender.clone(),
    }
  }

  pub fn clone_for_manager(
    &mut self,
    system_sender: mpsc::UnboundedSender<SystemMessage>,
    system_id: u64,
  ) -> Context {
    Context {
      store: self.store,
      system_id,
      app_sender: self.app_sender.clone(),
      system_sender,
    }
  }

  pub(crate) fn to_json(&self) -> Value {
    self.store.to_json()
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
