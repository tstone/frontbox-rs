use crate::mainboard_comms::{MainboardCommand, MainboardIncoming};
use bevy_ecs::resource::Resource;
use tokio::sync::mpsc;

#[derive(Debug, Resource)]
pub struct Mainboard {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  watchdog_enabled: bool,
}

#[derive(Debug, Clone)]
pub enum FastChannel {
  Io,
  Expansion,
}

impl Mainboard {
  pub fn new(
    command_tx: mpsc::Sender<MainboardCommand>,
    event_rx: mpsc::Receiver<MainboardIncoming>,
  ) -> Self {
    Mainboard {
      command_tx,
      event_rx,
      watchdog_enabled: false,
    }
  }

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

  pub fn receive(&mut self) -> Option<MainboardIncoming> {
    match self.event_rx.try_recv() {
      Ok(event) => Some(event),
      Err(_) => None,
    }
  }
}
