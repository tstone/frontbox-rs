//! A machine with no hardware behind it. It keeps the state real hardware would in a [`VirtualHardware`], and answers
//! the app as the hardware would.

mod virtual_hardware;

use tokio::sync::mpsc;

use crate::machine::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub use virtual_hardware::*;

pub struct VirtualMachine {
  hardware: VirtualHardware,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
}

impl VirtualMachine {
  fn process_message(&mut self, msg: MachineMessage) {
    log_message(&msg);
    for reply in self.hardware.apply(&msg) {
      self.app_sender.send(reply).ok();
    }
  }
}

impl MachineBoot for VirtualMachine {
  type Machine = VirtualMachine;

  async fn boot(
    boot_config: BootConfig,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> (Self::Machine, Hardware) {
    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();
    let virtual_hardware = VirtualHardware::new(&boot_config.io_network);

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
      // read at boot, as from real hardware
      SwitchLookup::new(boot_config.io_network.switches, virtual_hardware.switch_report()),
      DriverLookup::new(boot_config.io_network.drivers),
      LedLookup::new(&expansion_boards),
      io_network_boards,
      expansion_boards,
    );

    (
      Self {
        hardware: virtual_hardware,
        app_sender,
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

  /// Configure every driver from its mode, as the Neuron does before the app starts
  async fn on_pre_run(&mut self, state: &AppState) {
    for driver in state.drivers.values() {
      if let Some(mode) = state.drivers.mode(driver.name) {
        self.hardware.apply(&MachineMessage::ConfigureDriver {
          driver_id: driver.id,
          config: mode.to_config(state),
        });
      }
    }
  }

  async fn run(&mut self) {
    while let Some(msg) = self.machine_receiver.recv().await {
      self.process_message(msg);
    }
  }
}

fn log_message(msg: &MachineMessage) {
  match msg {
    MachineMessage::Dispatch { port, command } => {
      log::info!("VM: Dispatch on {:?} => {:?}", port, command);
    }
    MachineMessage::Command { port, command, .. } => {
      log::info!("VM: Request on {:?} => {:?}", port, command);
    }
    MachineMessage::ConfigureDriver { driver_id, config }
    | MachineMessage::UpdateDriverConfig { driver_id, config } => {
      log::info!("VM: Configure driver {} => {:?}", driver_id, config);
    }
    MachineMessage::TriggerDriver {
      driver_id,
      mode,
      switch,
    } => {
      log::info!("VM: Trigger driver {} => {:?} {:?}", driver_id, mode, switch);
    }
    _ => {} // watchdog, switch refresh
  }
}
