use tokio::sync::mpsc;

use crate::prelude::*;

pub struct VirtualMachine {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
}

impl VirtualMachine {
  async fn process_messages(&mut self, msg: MachineMessage) {
    match msg {
      MachineMessage::Dispatch { port, command } => {
        log::info!("VM: Dispatch on {:?} => {:?}", port, command);
      }
      MachineMessage::Command { port, command, .. } => {
        log::info!("VM: Request on {:?} => {:?}", port, command);
      }
      MachineMessage::RefreshSwitchState => {
        todo!()
      }
      _ => {} // watchdog, etc.
    }
  }
}

impl MachineBoot for VirtualMachine {
  type Machine = VirtualMachine;

  async fn boot(
    boot_config: BootConfig,
    _app_sender: mpsc::UnboundedSender<app_message::AppMessage>,
  ) -> (Self::Machine, Hardware) {
    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();

    let io_network_boards = boot_config
      .io_network
      .boards
      .iter()
      .enumerate()
      .map(|(idx, board)| ResolvedIoBoard {
        node_id: idx as u8,
        description: board.description,
        name: format!("Board {}", idx),
        firmware_version: "0.0.virtual".to_string(),
        board_revision: 0,
        switch_count: board.switch_count,
        driver_count: board.driver_count,
      })
      .collect::<Vec<_>>();

    let expansion_boards = Hardware::resolve_expansion_boards(&boot_config.exp_network.boards);

    let hardware = Hardware::new(
      SwitchLookup::new(boot_config.io_network.switches, Vec::new()),
      DriverLookup::new(boot_config.io_network.drivers),
      LedLookup::new(&expansion_boards),
      io_network_boards,
      expansion_boards,
    );

    (
      Self {
        machine_receiver,
        machine_sender,
      },
      hardware,
    )
  }
}

impl Machine for VirtualMachine {
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  async fn on_pre_run(&mut self, _snapshot: &BootSnapshot) {}

  async fn run(&mut self) {
    loop {
      tokio::select! {
        Some(msg) = self.machine_receiver.recv() => {
          self.process_messages(msg).await;
        }
      }
    }
  }
}
