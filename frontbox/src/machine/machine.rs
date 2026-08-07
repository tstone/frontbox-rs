use std::time::Duration;

use crate::app::app_message::EventBox;
use crate::hardware::*;
use crate::machine::serial_interface::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

pub(crate) struct MachineImpl {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
  watchdog_interval: Duration,
  switches: SwitchLookup,
  io_network: Vec<ResolvedIoBoard>,
}

impl MachineImpl {
  pub(crate) fn new(
    io_port: SerialInterface,
    exp_port: SerialInterface,
    app_sender: mpsc::UnboundedSender<AppMessage>,
    switches: SwitchLookup,
    io_network: Vec<ResolvedIoBoard>,
    app_config: AppConfig,
  ) -> Self {
    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();

    Self {
      io_port,
      exp_port,
      app_sender,
      machine_sender,
      machine_receiver,
      watchdog_interval: app_config.watchdog_interval + Duration::from_millis(250), // add some buffer to account for latency in sending
      switches,
      io_network,
    }
  }

  pub(crate) fn sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  pub async fn run(&mut self) {
    loop {
      tokio::select! {
        Some(event) = self.io_port.read_event() => {
          match event {
            EventResponse::Switch { switch_id, state } => {
              self.handle_switch_event(switch_id, state);
            }
          }
        }

        Some(msg) = self.machine_receiver.recv() => {
          self.process_messages(msg).await;
        }
      }
    }
  }

  fn port_for(&mut self, port: Port) -> &mut SerialInterface {
    match port {
      Port::Io => &mut self.io_port,
      Port::Exp => &mut self.exp_port,
    }
  }

  async fn process_messages(&mut self, msg: MachineMessage) {
    match msg {
      MachineMessage::WatchdogPing => {
        self.send_watchdog_ping().await;
      }
      MachineMessage::Dispatch { port, command } => {
        let port = self.port_for(port);
        port.dispatch(&*command).await;
      }
      MachineMessage::Request {
        port,
        command,
        timeout,
      } => {
        let port = self.port_for(port);
        port.request_any(&*command, timeout).await.ok();
      }
    }
  }

  pub fn handle_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    let switch = self.switches.by_id(&switch_id).cloned();

    if let Some(switch) = switch {
      // App needs to update switch state in the store before sending out the event
      self
        .app_sender
        .send(AppMessage::SingleSwitchState(switch_id, state))
        .ok();

      if matches!(state, SwitchState::Closed) {
        log::debug!("🎚️  Switch {} closed", switch.name);
        let event = SwitchClosed::new(switch);
        self
          .app_sender
          .send(AppMessage::EmitEvent(EventBox::new(event)))
          .ok();
      } else {
        log::debug!("🎚️  Switch {} opened", switch.name);
        let event = SwitchOpened::new(switch);
        self
          .app_sender
          .send(AppMessage::EmitEvent(EventBox::new(event)))
          .ok();
      }
    } else {
      // Report as native board/switch id since this is the easiest way to figure out current switch wiring
      match self.get_native_switch_id(switch_id) {
        Some((board_id, local_id)) => {
          log::warn!(
            "Received event for unknown switch -- board: {}, id: {} -- {:?}",
            board_id,
            local_id,
            state
          );
          return;
        }
        None => {
          log::warn!(
            "Received event for unknown switch on unknown board {} -- {:?}",
            switch_id,
            state
          );
        }
      }
      return;
    }
  }

  // ---

  pub async fn send_watchdog_ping(&mut self) {
    let _ = self
      .io_port
      .request(
        &WatchdogCommand::set(self.watchdog_interval),
        Duration::from_millis(200),
      )
      .await;
  }

  /// Primarily used for reporting of unknown switches as native board/switch ids
  fn get_native_switch_id(&self, switch_id: usize) -> Option<(usize, usize)> {
    let mut offset: usize = 0;
    for (index, board) in self.io_network.iter().enumerate() {
      if switch_id < (board.switch_count as usize) + offset {
        let native_switch_id = switch_id - offset;
        return Some((index, native_switch_id));
      }
      offset += board.switch_count as usize;
    }
    None
  }
}

/// Primary interface for interaction with FAST hardware
pub struct Machine {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
}

#[allow(unused)]
impl Machine {
  pub(crate) fn new(machine_sender: mpsc::UnboundedSender<MachineMessage>) -> Self {
    Self { machine_sender }
  }

  /// Ping the watchdog to prevent it from triggering a reset (WD)
  pub fn ping_watchdog(&self) {
    self.machine_sender.send(MachineMessage::WatchdogPing).ok();
  }

  /// Immediately expire the watchdog (WD:0)
  pub fn clear_watchdog(&self) {
    self
      .machine_sender
      .send(MachineMessage::Request {
        port: Port::Io,
        command: Box::new(WatchdogCommand::disable()),
        timeout: Duration::from_millis(200),
      })
      .ok();
  }

  pub fn reset_expansion_network(&self, ctx: &Context) {
    for board in ctx.exp_network.iter() {
      self
        .machine_sender
        .send(MachineMessage::Request {
          port: Port::Exp,
          command: Box::new(BoardResetCommand::new(board.address)),
          timeout: Duration::from_millis(200),
        })
        .ok();
    }
  }

  /// Configure a driver with a specific mode (e.g. enable with certain power level, or set to automatic) (DL)
  pub fn configure_driver(&self, driver: &str, mode: impl DriverMode + 'static, ctx: &Context) {
    if let Some(driver) = ctx.drivers.get(driver) {
      let config = mode.to_config(&ctx);
      self
        .machine_sender
        .send(MachineMessage::Request {
          port: Port::Io,
          command: Box::new(ConfigureDriverCommand::new(driver.id, config)),
          timeout: Duration::from_millis(200),
        })
        .ok();
    }
  }

  /// Activate a driver based on an activation mode (e.g. tap, automatic with switch, or virtual switch) (TL)
  pub fn activate_driver(&self, driver: &str, mode: ActivationMode, ctx: &Context) {
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
  pub fn deactivate_driver(&self, driver: &str, mode: DeactivationMode, ctx: &Context) {
    if let Some(driver) = ctx.drivers.get(driver) {
      let control_mode: DriverTriggerControlMode = match mode {
        DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
        DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
      };
      self.trigger_driver(driver.id, control_mode, None);
    }
  }

  fn trigger_driver(&self, driver: usize, mode: DriverTriggerControlMode, switch: Option<usize>) {
    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: Port::Io,
        command: Box::new(TriggerDriverCommand::new(driver, mode, switch)),
      })
      .ok();
  }

  /// Request the current state of all switches (SA). This will also automatically update Context with the latest switch states.
  pub fn refresh_switch_state(&self) {
    self
      .machine_sender
      .send(MachineMessage::Request {
        port: Port::Io,
        command: Box::new(ReportSwitchesCommand),
        timeout: Duration::from_secs(2),
      })
      .ok();
  }

  /// Configure a switch to report in a certain way (e.g. inverted, or with debounce) (SL)
  pub fn configure_switch(
    &self,
    switch: &str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
    ctx: &Context,
  ) {
    if let Some(switch) = ctx.switches.get(switch) {
      let reporting = if inverted {
        SwitchReportingMode::ReportInverted
      } else {
        SwitchReportingMode::ReportNormal
      };
      self
        .machine_sender
        .send(MachineMessage::Request {
          port: Port::Io,
          command: Box::new(ConfigureSwitchCommand::new(
            switch.id,
            reporting,
            debounce_close,
            debounce_open,
          )),
          timeout: Duration::from_millis(200),
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

    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: Port::Exp,
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
    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: Port::Exp,
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
    self
      .machine_sender
      .send(MachineMessage::Dispatch {
        port: Port::Exp,
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
        port: Port::Exp,
        command: Box::new(SetWhiteCommand::new(expansion_id, breakout, led_states)),
      })
      .ok();
  }
}

impl System for Machine {
  fn on_despawn(&mut self, ctx: &Context) {
    // Clear out LEDs, servos, etc.
    self.reset_expansion_network(ctx);

    // Disable drivers
    for driver in ctx.drivers.values() {
      self
        .machine_sender
        .send(MachineMessage::Request {
          port: Port::Io,
          command: Box::new(ConfigureDriverCommand::new(
            driver.id,
            DriverConfig::Disabled,
          )),
          timeout: Duration::from_millis(200),
        })
        .ok();
    }
  }
}

pub enum Port {
  Io,
  Exp,
}

pub enum MachineMessage {
  WatchdogPing,
  Dispatch {
    port: Port,
    command: Box<dyn FastBinaryCommand>,
  },
  Request {
    port: Port,
    command: Box<dyn FastAnyRequestCommand>,
    timeout: Duration,
  },
}