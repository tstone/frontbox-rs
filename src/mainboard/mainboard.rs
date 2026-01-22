use crate::mainboard_io::{MainboardCommand, MainboardIncoming};
use crate::prelude::{BootConfig, MainboardIO};
use bevy_ecs::resource::Resource;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Mainboard {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_tx: broadcast::Sender<MainboardIncoming>,
}

#[derive(Debug, Clone)]
pub enum FastChannel {
  Io,
  Expansion,
}

impl Mainboard {
  pub async fn boot(config: BootConfig) -> Self {
    let (command_tx, command_rx) = mpsc::channel::<MainboardCommand>(128);
    let (event_tx, _) = broadcast::channel::<MainboardIncoming>(128);

    // TODO: should event_rx be replaced by Tokio broadcast event bus?

    let mut mainboard = MainboardIO::boot(config, command_rx, event_tx.clone()).await;

    // start serial communication in separate thread
    std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

      runtime.block_on(async move {
        mainboard.run().await;
      });
    });

    Mainboard {
      command_tx,
      event_tx,
    }
  }

  pub fn send(&mut self, command: MainboardCommand) {
    self.command_tx.try_send(command).unwrap();
  }

  pub fn subscribe(&self) -> broadcast::Receiver<MainboardIncoming> {
    self.event_tx.subscribe()
  }

  pub fn tx(&self) -> mpsc::Sender<MainboardCommand> {
    self.command_tx.clone()
  }
}
