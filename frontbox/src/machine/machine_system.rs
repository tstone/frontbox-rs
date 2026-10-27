use std::collections::HashMap;
use std::time::Duration;

use crate::hardware::*;
use crate::machine::*;
use crate::operator_config::ConfigChanged;
use crate::prelude::app_message::AppMessage;
use crate::prelude::app_tracer::{DriverState, TraceEvent};
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

pub(crate) const COMMAND_TIMEOUT: Duration = Duration::from_millis(100);

/// Primary interface for interaction with FAST hardware
pub struct MachineSystem {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  /// The mode last sent to each driver. This is the only thing that sends DL at runtime, so unlike `ctx.drivers`,
  /// which catches up a pass later, it's never behind the board.
  driver_modes: HashMap<usize, DriverMode>,
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
      driver_modes: HashMap::new(),
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
  ///
  /// The driver has to be activated again to make it live. `ctx.drivers` shows the new mode from the next pass, and
  /// operator config changes are re-sent with it.
  pub fn configure_driver(&mut self, driver: &str, mode: DriverMode, ctx: &ServiceContext) {
    if let Some(driver) = ctx.drivers.get(driver) {
      self
        .machine_sender
        .send(MachineMessage::ConfigureDriver {
          driver_id: driver.id,
          config: mode.to_config(&ctx),
        })
        .ok();
      self.driver_modes.insert(driver.id, mode.clone());
      self
        .app_sender
        .send(AppMessage::DriverModeChange(driver.id, mode))
        .ok();
    }
  }

  /// Activate a driver based on an activation mode (e.g. tap, automatic with switch, or virtual switch) (TL)
  pub fn activate_driver(&self, driver: &str, mode: ActivationMode, ctx: &ServiceContext) {
    // remap switch to id
    let switch = mode.switch_name().and_then(|name| {
      let switch = ctx.switches.by_name(name).map(|sw| sw.id);
      if switch.is_none() {
        log::error!(
          "Activating driver \"{}\" with unknown switch \"{}\", keeping its configured switch",
          driver,
          name
        );
      }
      switch
    });
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
        DeactivationMode::Disabled => DriverTriggerControlMode::Off,
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
      .send(MachineMessage::TriggerDriver {
        driver_id: driver,
        mode,
        switch,
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

  /// Find the config by key, and update any drivers that use it
  fn update_drivers_on_config_change(&self, key: &'static str, ctx: &SystemContext) {
    for (driver_id, mode) in &self.driver_modes {
      if mode.configs().iter().any(|config| config.key == key) {
        self
          .machine_sender
          .send(MachineMessage::UpdateDriverConfig {
            driver_id: *driver_id,
            config: mode.to_config(ctx),
          })
          .ok();
      }
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

  /// Set the color of multiple LEDs in a single command (RD)
  pub fn set_leds(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u8, Rgba<u8>)>) {
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
        command: Box::new(SetLedsBinaryCommand::new(
          expansion_id,
          breakout,
          led_states,
        )),
      })
      .ok();
  }

  /// Set multiple LEDs on a port/breakout to the same color (RB)
  pub fn set_multiple_leds(
    &self,
    expansion_id: u8,
    breakout: Option<u8>,
    rgba: Rgba<u8>,
    indexes: Vec<u8>,
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
        command: Box::new(SetMultipleLedsBinaryCommand::new(
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

  /// Set the white channel of multiple LEDs in a single command (RZ)
  pub fn set_leds_white(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u8, u8)>) {
    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: MachinePort::Exp,
        command: Box::new(SetWhiteBinaryCommand::new(
          expansion_id,
          breakout,
          led_states,
        )),
      })
      .ok();
  }
}

impl System for MachineSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.driver_modes = ctx
      .drivers
      .modes()
      .map(|(driver_id, mode)| (driver_id, mode.clone()))
      .collect();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(ConfigChanged(key)) = event.downcast_ref::<ConfigChanged>() {
      self.update_drivers_on_config_change(key, ctx);
    }
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    // Clear out LEDs, servos, etc.
    self.reset_expansion_network(ctx.into());

    // Disable drivers
    for driver in ctx.drivers.values() {
      self
        .machine_sender
        .send(MachineMessage::ConfigureDriver {
          driver_id: driver.id,
          config: DriverConfig::Disabled,
        })
        .ok();
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use super::*;

  const COIL: &str = "coil";
  const COIL_ID: usize = 0xA;

  static POWER: LazyLock<Config<Power>> =
    LazyLock::new(|| Config::new("Coil Power", "", Power::FULL, Ranges::full_power()));
  static COIL_DEF: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new(COIL)
      .mode(pulse_using_power())
      .build()
  });
  static OTHER_DEF: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("other")
      .mode(DriverMode::pulse().build())
      .build()
  });

  fn pulse_using_power() -> DriverMode {
    DriverMode::pulse()
      .initial_pwm_power(HardwareValue::Config(POWER.clone()))
      .build()
  }

  struct Harness {
    ctx: TestContext,
    machine: MachineSystem,
    machine_rx: mpsc::UnboundedReceiver<MachineMessage>,
    app_rx: mpsc::UnboundedReceiver<AppMessage>,
  }

  impl Harness {
    fn new() -> Self {
      let mut ctx = TestContext::default();
      ctx.base.drivers = DriverLookup::new(vec![
        IoAddressed {
          definition: &*COIL_DEF,
          assignment: IoAddress::new(0, 0),
          id: COIL_ID,
        },
        IoAddressed {
          definition: &*OTHER_DEF,
          assignment: IoAddress::new(0, 1),
          id: 0xB,
        },
      ]);
      ctx.insert_switch(Switch {
        name: "sw",
        id: 0x12,
        ..Default::default()
      });

      let (machine_tx, machine_rx) = mpsc::unbounded_channel();
      let (app_tx, app_rx) = mpsc::unbounded_channel();
      let mut machine = MachineSystem::new(machine_tx, app_tx);
      machine.on_spawn(&ctx.sys_ctx());

      Self {
        ctx,
        machine,
        machine_rx,
        app_rx,
      }
    }

    fn configure(&mut self, mode: DriverMode) {
      self
        .machine
        .configure_driver(COIL, mode, &self.ctx.svc_ctx());
    }

    fn activate(&mut self, mode: ActivationMode) {
      self
        .machine
        .activate_driver(COIL, mode, &self.ctx.svc_ctx());
    }

    fn deactivate(&mut self, mode: DeactivationMode) {
      self
        .machine
        .deactivate_driver(COIL, mode, &self.ctx.svc_ctx());
    }

    /// Set the power and deliver the `ConfigChanged` the run loop would emit, returning what reached the machine
    fn change_power(&mut self, power: Power) -> Vec<MachineMessage> {
      self.sent();
      self.ctx.base.config.set(&POWER, power);
      self
        .machine
        .on_event(&ConfigChanged(POWER.display().key), &self.ctx.sys_ctx());
      self.sent()
    }

    fn sent(&mut self) -> Vec<MachineMessage> {
      let mut sent = Vec::new();
      while let Ok(msg) = self.machine_rx.try_recv() {
        sent.push(msg);
      }
      sent
    }

    /// The driver commands sent to the machine, in order: each TL as sent over the wire, and each DL as `DL:<driver>`
    fn driver_commands(&mut self) -> Vec<String> {
      self
        .sent()
        .into_iter()
        .filter_map(|msg| match msg {
          MachineMessage::TriggerDriver {
            driver_id,
            mode,
            switch,
          } => {
            let command = TriggerDriverCommand::new(driver_id, mode, switch);
            Some(String::from_utf8(command.to_bytes()).unwrap())
          }
          MachineMessage::ConfigureDriver { driver_id, .. } => Some(format!("DL:{:X}", driver_id)),
          _ => None,
        })
        .collect()
    }
  }

  #[test]
  fn deactivate_disabled_disconnects_switch() {
    // TL control 2 turns the driver off and disconnects its switch. Control 0 would do the opposite and
    // connect it, leaving e.g. pops and slings live after the turn ends.
    let mut harness = Harness::new();
    harness.deactivate(DeactivationMode::Disabled);
    assert_eq!(harness.driver_commands(), vec!["TL:A,2,\r"]);
  }

  #[test]
  fn deactivate_virtual_switch_off() {
    let mut harness = Harness::new();
    harness.deactivate(DeactivationMode::VirtualSwitchOff);
    assert_eq!(harness.driver_commands(), vec!["TL:A,2,\r"]);
  }

  #[test]
  fn activate_automatic_connects_switch() {
    let mut harness = Harness::new();
    harness.activate(ActivationMode::Automatic("sw"));
    assert_eq!(harness.driver_commands(), vec!["TL:A,0,12\r"]);
  }

  #[test]
  fn activate_tap_and_on() {
    let mut harness = Harness::new();
    harness.activate(ActivationMode::Tap);
    harness.activate(ActivationMode::VirtualSwitchOn);
    assert_eq!(harness.driver_commands(), vec!["TL:A,1,\r", "TL:A,3,\r"]);
  }

  #[test]
  fn configure_then_activate_stays_in_order() {
    let mut harness = Harness::new();
    harness.configure(DriverMode::pulse().build());
    harness.activate(ActivationMode::Automatic("sw"));
    assert_eq!(harness.driver_commands(), vec!["DL:A", "TL:A,0,12\r"]);
  }

  #[test]
  fn configuring_tells_the_run_loop_the_new_mode() {
    let mut harness = Harness::new();
    harness.configure(DriverMode::long_pulse().build());

    let changes: Vec<_> = std::iter::from_fn(|| harness.app_rx.try_recv().ok())
      .filter_map(|msg| match msg {
        AppMessage::DriverModeChange(driver_id, mode) => Some((driver_id, mode)),
        _ => None,
      })
      .collect();
    assert!(matches!(changes[..], [(COIL_ID, DriverMode::LongPulse(_))]));
  }

  #[test]
  fn a_config_change_updates_only_drivers_using_the_key() {
    let mut harness = Harness::new();
    let sent = harness.change_power(Power::HALF);

    assert!(matches!(
      sent[..],
      [MachineMessage::UpdateDriverConfig {
        driver_id: COIL_ID,
        config: DriverConfig::Pulse { initial_pwm_power, .. }
      }] if initial_pwm_power == Power::HALF
    ));
  }

  #[test]
  fn a_config_change_resends_the_last_mode_sent() {
    // `ctx.drivers` still has the defined mode here, as it would while the run loop catches up
    let mut harness = Harness::new();
    harness.configure(
      DriverMode::long_pulse()
        .initial_pwm_power(HardwareValue::Config(POWER.clone()))
        .build(),
    );

    let sent = harness.change_power(Power::HALF);
    assert!(matches!(
      sent[..],
      [MachineMessage::UpdateDriverConfig {
        driver_id: COIL_ID,
        config: DriverConfig::LongPulse { initial_pwm_power, .. }
      }] if initial_pwm_power == Power::HALF
    ));
  }
}
