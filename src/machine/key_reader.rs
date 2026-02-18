use crossterm::event::EventStream;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;

pub fn monitor_keys(sender: mpsc::UnboundedSender<MachineCommand>) {
  tokio::spawn(async move {
    let mut key_reader = EventStream::new();
    loop {
      if let Some(Ok(event)) = key_reader.next().await {
        // Relay key events back to machine for processing
        sender.send(MachineCommand::Key(event)).ok();
      }
    }
  });
}
