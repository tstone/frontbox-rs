use crate::machine::*;
use crate::prelude::*;
use fast_protocol::BoardResetCommand;
use tokio::sync::mpsc;

/// What to send the machine if the run loop dies: expire the watchdog, which cuts power to the coils, and reset the
/// expansion boards, which blanks the LEDs. Built before the run loop takes the app state.
pub(crate) struct EmergencyStop {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  exp_addresses: Vec<u8>,
}

impl EmergencyStop {
  /// How long the machine task gets to send the power down messages before the process goes
  const FLUSH_TIME: Duration = Duration::from_millis(500);

  pub(crate) fn new(
    machine_sender: mpsc::UnboundedSender<MachineMessage>,
    state: &AppState,
  ) -> Self {
    Self {
      machine_sender,
      exp_addresses: state
        .exp_network
        .iter()
        .map(|board| board.address)
        .collect(),
    }
  }

  pub(crate) async fn run(&self) {
    let _ = self.machine_sender.send(MachineMessage::WatchdogClear);
    for address in &self.exp_addresses {
      let _ = self.machine_sender.send(MachineMessage::Command {
        port: MachinePort::Exp,
        command: Box::new(BoardResetCommand::new(*address)),
        timeout: COMMAND_TIMEOUT,
      });
    }
    // the machine runs on its own task, which keeps going while this one waits
    tokio::time::sleep(Self::FLUSH_TIME).await;
  }
}
