use crate::mainboard_comms::MainboardCommand;
use bevy_ecs::resource::Resource;
use tokio::sync::mpsc;

#[derive(Debug, Resource)]
pub struct Mainboard {
  pub command_tx: mpsc::Sender<MainboardCommand>,
  pub watchdog_enabled: bool,
}

impl Mainboard {
  pub fn enable_watchdog(&mut self) {
    if !self.watchdog_enabled {
      self.watchdog_enabled = true;
      let _ = self.command_tx.try_send(MainboardCommand::Watchdog(true));
    }
  }

  pub fn disable_watchdog(&mut self) {
    if self.watchdog_enabled {
      self.watchdog_enabled = false;
      let _ = self.command_tx.try_send(MainboardCommand::Watchdog(false));
    }
  }
}
