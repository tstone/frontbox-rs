use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub trait MachineBoot {
  type Machine: Machine;

  fn boot(
    boot_config: BootConfig,
    app: mpsc::UnboundedSender<AppMessage>,
  ) -> impl Future<Output = (Self::Machine, Hardware)>;
}

pub trait Machine: Send {
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage>;
  fn run<'a>(&'a mut self) -> impl Future<Output = ()>;
}
