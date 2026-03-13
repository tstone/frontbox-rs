use std::time::Duration;

use fast_protocol::{DriverConfig, DriverTriggerControlMode};
use tokio::sync::mpsc;

use crate::prelude::MachineCommand;

#[derive(Clone)]
pub struct DriverCommands {
  pub(crate) machine: mpsc::UnboundedSender<MachineCommand>,
}

impl DriverCommands {
  pub fn new(machine: mpsc::UnboundedSender<MachineCommand>) -> Self {
    Self { machine }
  }

  // TODO: make this equivalent with the IoBoard builder so that it takes names not IDs
  pub fn reconfigure(&mut self, driver_name: &'static str, config: DriverConfig) {
    let _ = self
      .machine
      .send(MachineCommand::ConfigureDriver(driver_name, config));
  }

  /// Activate (trigger) a driver with the given mode. This emits `TL` commands to the FAST hardware
  pub fn activate(&mut self, driver_name: &'static str, mode: ActivationMode) {
    let control_mode: DriverTriggerControlMode = match mode {
      ActivationMode::Automatic => DriverTriggerControlMode::Automatic,
      ActivationMode::Tap => DriverTriggerControlMode::Manual,
      ActivationMode::VirtualSwitchOn => DriverTriggerControlMode::On,
    };
    let _ = self.machine.send(MachineCommand::TriggerDriver(
      driver_name,
      control_mode,
      None,
    ));
  }

  /// Deactivate a driver with the given mode. This emits `TL` commands to the FAST hardware
  pub fn deactivate(&mut self, driver_name: &'static str, mode: DeactivationMode) {
    let control_mode: DriverTriggerControlMode = match mode {
      DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
      DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
    };
    let _ = self.machine.send(MachineCommand::TriggerDriver(
      driver_name,
      control_mode,
      None,
    ));
  }

  pub fn trigger(&mut self, driver_name: &'static str, mode: DriverTriggerControlMode) {
    let _ = self
      .machine
      .send(MachineCommand::TriggerDriver(driver_name, mode, None));
  }

  /// Triggers a driver after the given delay time has elapsed
  pub fn trigger_delayed(
    &mut self,
    driver_name: &'static str,
    mode: DriverTriggerControlMode,
    delay: Duration,
  ) {
    let _ = self.machine.send(MachineCommand::TriggerDriver(
      driver_name,
      mode,
      Some(delay),
    ));
  }
}

pub enum ActivationMode {
  /// let the machine decide when to trigger this driver based on its configured trigger
  Automatic,
  /// manually trigger (activate) the driver immediately
  Tap,
  /// set virtual switch to 'on' for hold trigger modes
  VirtualSwitchOn,
}

pub enum DeactivationMode {
  Disabled,
  /// set virtual switch to 'off' for hold trigger modes
  VirtualSwitchOff,
}
