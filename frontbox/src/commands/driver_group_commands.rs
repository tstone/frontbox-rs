use std::time::Duration;

use crate::commands::driver_commands::*;
use fast_protocol::DriverTriggerControlMode;
use tokio::sync::mpsc;

use crate::prelude::MachineCommand;

#[derive(Clone)]
pub struct DriverGroupCommands {
  pub(crate) machine: mpsc::UnboundedSender<MachineCommand>,
}

impl DriverGroupCommands {
  pub fn new(machine: mpsc::UnboundedSender<MachineCommand>) -> Self {
    Self { machine }
  }

  /// Activate (trigger) a driver with the given mode. This emits `TL` commands to the FAST hardware
  pub fn activate(&mut self, group_name: &'static str, mode: ActivationMode) {
    let control_mode: DriverTriggerControlMode = match mode {
      ActivationMode::Automatic => DriverTriggerControlMode::Automatic,
      ActivationMode::Tap => DriverTriggerControlMode::Manual,
      ActivationMode::VirtualSwitchOn => DriverTriggerControlMode::On,
    };
    let _ = self.machine.send(MachineCommand::TriggerDriverGroup(
      group_name,
      control_mode,
      None,
    ));
  }

  /// Deactivate a driver with the given mode. This emits `TL` commands to the FAST hardware
  pub fn deactivate(&mut self, group_name: &'static str, mode: DeactivationMode) {
    let control_mode: DriverTriggerControlMode = match mode {
      DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
      DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
    };
    let _ = self.machine.send(MachineCommand::TriggerDriverGroup(
      group_name,
      control_mode,
      None,
    ));
  }

  pub fn trigger(&mut self, group_name: &'static str, mode: DriverTriggerControlMode) {
    let _ = self
      .machine
      .send(MachineCommand::TriggerDriverGroup(group_name, mode, None));
  }

  /// Triggers a driver after the given delay time has elapsed
  pub fn trigger_delayed(
    &mut self,
    group_name: &'static str,
    mode: DriverTriggerControlMode,
    delay: Duration,
  ) {
    let _ = self.machine.send(MachineCommand::TriggerDriverGroup(
      group_name,
      mode,
      Some(delay),
    ));
  }
}
