use tokio::sync::mpsc;

use crate::prelude::MachineCommand;

#[derive(Clone)]
pub struct GameCommands {
  pub(crate) machine: mpsc::UnboundedSender<MachineCommand>,
}

impl GameCommands {
  pub fn new(machine: mpsc::UnboundedSender<MachineCommand>) -> Self {
    Self { machine }
  }
  pub fn start(&mut self) {
    let _ = self.machine.send(MachineCommand::StartGame);
  }

  pub fn end(&mut self) {
    let _ = self.machine.send(MachineCommand::EndGame);
  }

  pub fn add_player(&mut self) {
    let _ = self.machine.send(MachineCommand::AddPlayer);
  }

  pub fn advance_player(&mut self) {
    let _ = self.machine.send(MachineCommand::AdvancePlayer);
  }
}
