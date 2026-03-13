use tokio::sync::mpsc;

use crate::commands::driver_commands::*;
use crate::commands::driver_group_commands::*;
use crate::commands::game_commands::*;
use crate::commands::system_commands::*;
use crate::commands::timer_commands::*;
use crate::commands::writeable_config::*;
use crate::commands::writeable_store::*;
use crate::machine::machine_command::MachineCommand;
use crate::prelude::*;
use crate::systems::*;

/// Commands enqueues mutable actions
pub struct Commands {
  machine: mpsc::UnboundedSender<MachineCommand>,
  system_manager: mpsc::UnboundedSender<SystemCommand>,
  listener_id: u64,
  pub driver: DriverCommands,
  pub driver_group: DriverGroupCommands,
  pub game: GameCommands,
  pub system: SystemCommands,
  pub timer: TimerCommands,
  pub store: WriteableStore,
  pub config: WriteableConfig,
}

impl Commands {
  pub fn new(
    machine: mpsc::UnboundedSender<MachineCommand>,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store_manager: mpsc::UnboundedSender<StoreCommand>,
    listener_id: u64,
  ) -> Self {
    let config = WriteableConfig::new(machine.clone());
    Self {
      listener_id,
      game: GameCommands {
        machine: machine.clone(),
      },
      driver: DriverCommands {
        machine: machine.clone(),
      },
      driver_group: DriverGroupCommands {
        machine: machine.clone(),
      },
      system: SystemCommands {
        system_manager: system_manager.clone(),
        listener_id,
      },
      timer: TimerCommands {
        system_manager: system_manager.clone(),
        listener_id,
      },
      store: WriteableStore::new(store_manager),
      config,
      machine,
      system_manager,
    }
  }

  pub fn clone_for_system(&self, listener_id: u64) -> Self {
    Self::new(
      self.machine.clone(),
      self.system_manager.clone(),
      self.store.sender.clone(),
      listener_id,
    )
  }

  pub fn clone_for_manager(
    &self,
    system_manager: mpsc::UnboundedSender<SystemCommand>,
    store_manager: mpsc::UnboundedSender<StoreCommand>,
  ) -> Self {
    Self::new(
      self.machine.clone(),
      system_manager,
      store_manager,
      self.listener_id,
    )
  }

  pub fn transition(&mut self, new_state: impl StorableType + 'static) {
    let _ = self
      .machine
      .send(MachineCommand::StateTransition(Box::new(move |states| {
        states.set(new_state);
      })));
  }

  /// Broadcast an event to all listeners
  pub fn emit(&mut self, event: Box<dyn FrontboxEvent>) {
    let _ = self.machine.send(MachineCommand::EmitEvent(event));
  }
}
