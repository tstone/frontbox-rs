use std::time::Duration;

use crate::hardware::*;
use crate::machine::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::app_tracer::{DriverState, TraceEvent};
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

const COMMAND_TIMEOUT: Duration = Duration::from_millis(100);

/// Primary interface for interaction with FAST hardware
pub struct MachineSystem {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
}

#[allow(unused)]
impl MachineSystem {
  pub(crate) fn new(
    machine_sender: mpsc::UnboundedSender<MachineMessage>,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      machine_sender,
      app_sender,
    }
  }

  /// Ping the watchdog to prevent it from triggering a reset (WD)
  pub fn ping_watchdog(&self) {
    self.machine_sender.send(MachineMessage::WatchdogPing).ok();
  }

  /// Immediately expire the watchdog (WD:0)
  pub fn clear_watchdog(&self) {
    self.machine_sender.send(MachineMessage::WatchdogClear).ok();
  }

  pub fn reset_expansion_network(&self, ctx: &ServiceContext) {
    for board in ctx.exp_network.iter() {
      self
        .machine_sender
        .send(MachineMessage::Command {
          port: MachinePort::Exp,
          command: Box::new(BoardResetCommand::new(board.address)),
          timeout: COMMAND_TIMEOUT,
        })
        .ok();
    }
  }

  /// Configure a driver with a specific mode (e.g. enable with certain power level, or set to automatic) (DL)
  pub fn configure_driver(&self, driver: &str, mode: DriverMode, ctx: &ServiceContext) {
    if let Some(driver) = ctx.drivers.get(driver) {
      let config = mode.to_config(&ctx);
      self
        .machine_sender
        .send(MachineMessage::Command {
          port: MachinePort::Io,
          command: Box::new(ConfigureDriverCommand::new(driver.id, config)),
          timeout: COMMAND_TIMEOUT,
        })
        .ok();
    }
  }

  /// Activate a driver based on an activation mode (e.g. tap, automatic with switch, or virtual switch) (TL)
  pub fn activate_driver(&self, driver: &str, mode: ActivationMode, ctx: &ServiceContext) {
    // remap switch to id
    let switch = mode
      .switch_name()
      .and_then(|sw| ctx.switches.by_name(sw))
      .map(|sw| sw.id);
    if let Some(driver) = ctx.drivers.get(driver) {
      let control_mode: DriverTriggerControlMode = match mode {
        ActivationMode::Automatic(_) => DriverTriggerControlMode::Automatic,
        ActivationMode::Tap => DriverTriggerControlMode::Manual,
        ActivationMode::VirtualSwitchOn => DriverTriggerControlMode::On,
      };
      self.trigger_driver(driver.id, control_mode, switch);
    }
  }

  /// Deactivate a driver based on a deactivation mode (e.g. automatic, or virtual switch) (TL)
  pub fn deactivate_driver(&self, driver: &str, mode: DeactivationMode, ctx: &ServiceContext) {
    if let Some(driver) = ctx.drivers.get(driver) {
      let control_mode: DriverTriggerControlMode = match mode {
        DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
        DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
      };
      self.trigger_driver(driver.id, control_mode, None);
    }
  }

  fn trigger_driver(&self, driver: usize, mode: DriverTriggerControlMode, switch: Option<usize>) {
    let driver_state = match &mode {
      DriverTriggerControlMode::On => Some(DriverState::On),
      DriverTriggerControlMode::Off => Some(DriverState::Off),
      DriverTriggerControlMode::Manual => Some(DriverState::Fired),
      _ => None,
    };

    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Io,
        command: Box::new(TriggerDriverCommand::new(driver, mode, switch)),
      })
      .ok();

    if let Some(state) = driver_state {
      self
        .app_sender
        .send(AppMessage::TracerEvent(TraceEvent::DriverStateChange {
          driver_id: driver,
          state,
        }))
        .ok();
    }
  }

  /// Request the current state of all switches (SA). This will also automatically update Context with the latest switch states.
  pub fn refresh_switch_state(&self) {
    self
      .machine_sender
      .send(MachineMessage::RefreshSwitchState)
      .ok();
  }

  /// Configure a switch to report in a certain way (e.g. inverted, or with debounce) (SL)
  pub fn configure_switch(
    &self,
    switch: &str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
    ctx: &ServiceContext,
  ) {
    if let Some(switch) = ctx.switches.get(switch) {
      let reporting = if inverted {
        SwitchReportingMode::ReportInverted
      } else {
        SwitchReportingMode::ReportNormal
      };
      self
        .machine_sender
        .send(MachineMessage::Command {
          port: MachinePort::Io,
          command: Box::new(ConfigureSwitchCommand::new(
            switch.id,
            reporting,
            debounce_close,
            debounce_open,
          )),
          timeout: COMMAND_TIMEOUT,
        })
        .ok();
    }
  }

  /// Set the color of multiple LEDs in a single command (RS)
  pub fn set_leds(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u16, Rgba<u8>)>) {
    let led_states = led_states
      .iter()
      .map(|(index, color)| (*index, color.to_color()))
      .collect::<Vec<_>>();

    if cfg!(debug_assertions) {
      self
        .app_sender
        .send(AppMessage::TracerEvent(TraceEvent::LedsRGBChange {
          expansion: expansion_id,
          breakout,
          states: led_states.clone(),
        }))
        .ok();
    }

    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Exp,
        command: Box::new(SetLedsCommand::new(expansion_id, breakout, led_states)),
      })
      .ok();
  }

  /// Set all LEDs on a port/breakout to the same color (RP)
  pub fn set_multiple_leds(
    &self,
    expansion_id: u8,
    breakout: Option<u8>,
    rgba: Rgba<u8>,
    indexes: Vec<u16>,
  ) {
    if cfg!(debug_assertions) {
      self
        .app_sender
        .send(AppMessage::TracerEvent(TraceEvent::LedsRGBChange {
          expansion: expansion_id,
          breakout,
          states: indexes
            .clone()
            .into_iter()
            .map(|index| (index, rgba.to_color()))
            .collect(),
        }))
        .ok();
    }

    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Exp,
        command: Box::new(SetMultipleLedsCommand::new(
          expansion_id,
          breakout,
          rgba.to_color(),
          indexes,
        )),
      })
      .ok();
  }

  /// Set all LEDs on a port/breakout to the same color (RA)
  pub fn set_all_leds(&self, expansion_id: u8, breakout: Option<u8>, rgba: Rgba<u8>) {
    // TODO: tracer support

    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Exp,
        command: Box::new(SetAllLedsCommand::new(
          expansion_id,
          breakout,
          rgba.to_color(),
        )),
      })
      .ok();
  }

  /// Set the white channel of multiple LEDs in a single command (RW)
  pub fn set_leds_white(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u16, u8)>) {
    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Exp,
        command: Box::new(SetWhiteCommand::new(expansion_id, breakout, led_states)),
      })
      .ok();
  }
}

impl System for MachineSystem {
  fn on_despawn(&mut self, ctx: &SystemContext) {
    // Clear out LEDs, servos, etc.
    self.reset_expansion_network(ctx.into());

    // Disable drivers
    for driver in ctx.drivers.values() {
      self
        .machine_sender
        .send(MachineMessage::Command {
          port: MachinePort::Io,
          command: Box::new(ConfigureDriverCommand::new(
            driver.id,
            DriverConfig::Disabled,
          )),
          timeout: COMMAND_TIMEOUT,
        })
        .ok();
    }
  }
}
