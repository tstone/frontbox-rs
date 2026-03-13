use tokio::sync::mpsc;

use crate::prelude::*;
use crate::systems::SystemCommand;

pub struct TimerCommands {
  pub(crate) system_manager: mpsc::UnboundedSender<SystemCommand>,
  pub(crate) listener_id: u64,
}

impl TimerCommands {
  pub fn new(system_manager: mpsc::UnboundedSender<SystemCommand>, listener_id: u64) -> Self {
    Self {
      system_manager,
      listener_id,
    }
  }

  pub fn set(&mut self, timer_name: &'static str, duration: Duration, mode: TimerMode) {
    let _ = self.system_manager.send(SystemCommand::SetTimer(
      self.listener_id,
      timer_name,
      duration,
      mode,
    ));
  }

  pub fn clear(&mut self, timer_name: &'static str) {
    let _ = self
      .system_manager
      .send(SystemCommand::ClearTimer(self.listener_id, timer_name));
  }
}
