use tokio::sync::mpsc;

use crate::prelude::*;
use crate::systems::SystemCommand;

pub struct SystemCommands {
  pub(crate) system_manager: mpsc::UnboundedSender<SystemCommand>,
  pub(crate) listener_id: u64,
}

impl SystemCommands {
  pub fn new(system_manager: mpsc::UnboundedSender<SystemCommand>, listener_id: u64) -> Self {
    Self {
      system_manager,
      listener_id,
    }
  }

  pub fn spawn(&mut self, system: impl System + 'static) {
    let _ = self
      .system_manager
      .send(SystemCommand::SpawnSystem(Box::new(system)));
  }

  pub fn replace(&mut self, system: impl System + 'static) {
    let _ = self.system_manager.send(SystemCommand::ReplaceSystem(
      self.listener_id,
      Box::new(system),
    ));
  }

  pub fn despawn(&mut self) {
    let _ = self
      .system_manager
      .send(SystemCommand::DespawnSystem(self.listener_id));
  }
}
