use tokio::sync::mpsc;

use crate::machine::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub trait MachineBoot {
  type Machine: Machine;

  /// Handle all the hardware state initialization, port setup, handshakes, etc.
  fn boot(
    boot_config: BootConfig,
    app: mpsc::UnboundedSender<AppMessage>,
  ) -> impl Future<Output = (Self::Machine, Hardware)>;
}

pub trait Machine: Send {
  /// Machine must respond to all MachineMessages at this channel
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage>;
  /// Activate any hardware which requires configuration or to know the final state prior to start-up (e.g. drivers that read from operator config)
  fn on_pre_run(&mut self, snapshot: &BootSnapshot) -> impl Future<Output = ()>;
  /// Start the loop to listen for MachineMessages
  fn run<'a>(&'a mut self) -> impl Future<Output = ()>;
}
