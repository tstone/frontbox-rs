use tokio::sync::mpsc;

use crate::prelude::*;

#[derive(Clone)]
pub struct WriteableConfig {
  sender: mpsc::UnboundedSender<MachineCommand>,
}

impl WriteableConfig {
  pub fn new(sender: mpsc::UnboundedSender<MachineCommand>) -> Self {
    Self { sender }
  }

  pub fn set(&mut self, key: &'static str, value: ConfigValue) {
    let _ = self.sender.send(MachineCommand::SetConfigValue(key, value));
  }
}
