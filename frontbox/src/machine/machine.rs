use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

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
  context_base: ContextBase,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
  watchdog_interval: Duration,
  led_renderer: LedRenderer,
}

impl MachineImpl {
  pub(crate) fn new(
    io_port: SerialInterface,
    exp_port: SerialInterface,
    context_base: ContextBase,
    app_sender: mpsc::UnboundedSender<AppMessage>,
    led_renderer: LedRenderer,
  ) -> Self {
    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();

    Self {
      io_port,
      exp_port,
      app_sender,
      machine_sender,
      machine_receiver,
      watchdog_interval: context_base.app_config.watchdog_interval + Duration::from_millis(250), // add some buffer to account for latency in sending
      context_base,
      led_renderer,
    }
  }

  pub(crate) fn machine_sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
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

  async fn process_messages(&mut self, msg: MachineMessage) {
    match msg {
      MachineMessage::WatchdogPing => {
        self.send_watchdog_ping().await;
      }
      MachineMessage::ClearWatchdog => {
        self.clear_watchdog().await;
      }
      MachineMessage::ResetExpansionNetwork => {
        self.reset_expansion_network().await;
      }
      MachineMessage::ConfigureDriver(driver_id, config) => {
        self.configure_driver(driver_id, config).await;
      }
      MachineMessage::ActivateDriver(driver_id, mode, switch) => {
        self.activate_driver(driver_id, mode, switch).await;
      }
      MachineMessage::DeactivateDriver(driver_id, mode) => {
        self.deactivate_driver(driver_id, mode).await;
      }
      MachineMessage::ReportSwitches => {
        self.report_switches().await;
      }
      MachineMessage::RenderLedDeclarations(declarations) => {
        self
          .led_renderer
          .tick(self.context_base.app_config.system_interval);
        self
          .led_renderer
          .render(&mut self.exp_port, declarations)
          .await;
      }
      MachineMessage::ConfigureSwitch(switch_id, inverted, debounce_close, debounce_open) => {
        self
          .configure_switch(switch_id, inverted, debounce_close, debounce_open)
          .await;
      }
    }
  }

  pub fn handle_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    let switch = self.context_base.switches.switch_by_id(&switch_id).cloned();

    if let Some(switch) = switch {
      // App needs to update switch state in the store before sending out the event
      self
        .app_sender
        .send(AppMessage::SingleSwitchState(switch_id, state))
        .ok();

      if matches!(state, SwitchState::Closed) {
        self
          .app_sender
          .send(AppMessage::EmitEvent(Box::new(SwitchClosed::new(switch))))
          .ok();
      } else {
        self
          .app_sender
          .send(AppMessage::EmitEvent(Box::new(SwitchOpened::new(switch))))
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

  pub async fn clear_watchdog(&mut self) {
    let _ = self
      .io_port
      .request(&WatchdogCommand::disable(), Duration::from_millis(200))
      .await;
  }

  /// Primarily used for reporting of unknown switches as native board/switch ids
  fn get_native_switch_id(&self, switch_id: usize) -> Option<(usize, usize)> {
    let mut offset: usize = 0;
    for (index, board) in self.context_base.io_network.iter().enumerate() {
      if switch_id < (board.switch_count as usize) + offset {
        let native_switch_id = switch_id - offset;
        return Some((index, native_switch_id));
      }
      offset += board.switch_count as usize;
    }
    None
  }

  async fn configure_driver(&mut self, driver: usize, config: DriverConfig) {
    log::info!("Configuring driver {}", driver);
    match self
      .io_port
      .request(
        &ConfigureDriverCommand::new(&driver, &config),
        Duration::from_millis(200),
      )
      .await
    {
      Ok(ProcessedResponse::Failed) => {
        log::error!("Driver {} configuration failed", driver);
      }
      Err(e) => {
        log::error!("Error configuring driver {}: {}", driver, e);
      }
      _ => {}
    }
  }

  async fn report_switches(&mut self) {
    match self
      .io_port
      .request(&ReportSwitchesCommand::new(), Duration::from_secs(2))
      .await
    {
      Ok(SwitchReportResponse::SwitchReport { switches }) => {
        self
          .app_sender
          .send(AppMessage::SwitchStates(switches))
          .ok();
      }
      _ => {
        log::error!("Failed to report switches");
      }
    }
  }

  pub async fn activate_driver(
    &mut self,
    driver: usize,
    mode: ActivationMode,
    switch: Option<usize>,
  ) {
    log::info!("Activating driver {} with mode {:?}", driver, mode);
    let control_mode: DriverTriggerControlMode = match mode {
      ActivationMode::Automatic(_) => DriverTriggerControlMode::Automatic,
      ActivationMode::Tap => DriverTriggerControlMode::Manual,
      ActivationMode::VirtualSwitchOn => DriverTriggerControlMode::On,
    };
    self.trigger_driver(driver, control_mode, switch).await;
  }

  pub async fn deactivate_driver(&mut self, driver: usize, mode: DeactivationMode) {
    log::info!("Deactivating driver {} with mode {:?}", driver, mode);
    let control_mode: DriverTriggerControlMode = match mode {
      DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
      DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
    };
    self.trigger_driver(driver, control_mode, None).await;
  }

  async fn trigger_driver(
    &mut self,
    driver: usize,
    mode: DriverTriggerControlMode,
    switch: Option<usize>,
  ) {
    log::info!("Triggering driver {}", driver);
    self
      .io_port
      .dispatch(&TriggerDriverCommand::new(driver, mode, switch))
      .await;
  }

  pub async fn reset_expansion_network(&mut self) {
    self.led_renderer.reset();

    for board in self.context_base.exp_network.iter() {
      // TODO: move this to a better common location
      App::reset_expansion_board(&mut self.exp_port, board).await;
    }
  }

  async fn configure_switch(
    &mut self,
    switch: usize,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
  ) {
    log::info!("Configuring switch {}", switch);
    let reporting = if inverted {
      SwitchReportingMode::ReportInverted
    } else {
      SwitchReportingMode::ReportNormal
    };
    match self
      .io_port
      .request(
        &ConfigureSwitchCommand::new(switch, reporting, debounce_close, debounce_open),
        Duration::from_millis(200),
      )
      .await
    {
      Ok(ProcessedResponse::Failed) => {
        log::error!("Switch {} configuration failed", switch);
      }
      Err(e) => {
        log::error!("Error configuring switch {}: {}", switch, e);
      }
      _ => {}
    }
  }
}

/// Primary interface for sending commands to the machine and receiving machine commands
pub struct Machine {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
}

#[allow(unused)]
impl Machine {
  pub(crate) fn new(machine_sender: mpsc::UnboundedSender<MachineMessage>) -> Self {
    Self { machine_sender }
  }

  pub fn ping_watchdog(&self) {
    self.machine_sender.send(MachineMessage::WatchdogPing).ok();
  }

  pub fn clear_watchdog(&self) {
    self.machine_sender.send(MachineMessage::ClearWatchdog).ok();
  }

  pub fn reset_expansion_network(&self) {
    self
      .machine_sender
      .send(MachineMessage::ResetExpansionNetwork)
      .ok();
  }

  pub fn configure_driver(
    &self,
    driver: &'static str,
    mode: impl DriverMode + 'static,
    ctx: &Context,
  ) {
    if let Some(driver) = ctx.drivers.get(driver) {
      let config = mode.to_config(&ctx.switches);
      self
        .machine_sender
        .send(MachineMessage::ConfigureDriver(driver.id, config))
        .ok();
    }
  }

  pub fn activate_driver(&self, driver: &'static str, mode: ActivationMode, ctx: &Context) {
    if let Some(driver) = ctx.drivers.get(driver) {
      self
        .machine_sender
        .send(MachineMessage::ActivateDriver(driver.id, mode, None))
        .ok();
    }
  }

  pub fn deactivate_driver(&self, driver: &'static str, mode: DeactivationMode, ctx: &Context) {
    if let Some(driver) = ctx.drivers.get(driver) {
      self
        .machine_sender
        .send(MachineMessage::DeactivateDriver(driver.id, mode))
        .ok();
    }
  }

  pub fn refresh_switch_state(&self) {
    self
      .machine_sender
      .send(MachineMessage::ReportSwitches)
      .ok();
  }

  pub fn configure_switch(
    &self,
    switch: &'static str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
    ctx: &Context,
  ) {
    if let Some(switch) = ctx.switches.get(switch) {
      self
        .machine_sender
        .send(MachineMessage::ConfigureSwitch(
          switch.id,
          inverted,
          debounce_close,
          debounce_open,
        ))
        .ok();
    }
  }
}

impl System for Machine {
  fn on_shutdown(&mut self, ctx: &Context, _systems: &Systems) {
    for driver in ctx.drivers.values() {
      self
        .machine_sender
        .send(MachineMessage::DeactivateDriver(
          driver.id,
          DeactivationMode::Disabled,
        ))
        .ok();
    }

    // TODO: disable/unconfigure drivers
    self
      .machine_sender
      .send(MachineMessage::ResetExpansionNetwork)
      .ok();
  }
}

#[derive(Debug)]
pub enum MachineMessage {
  WatchdogPing,
  ClearWatchdog,
  ReportSwitches,
  ResetExpansionNetwork,
  ConfigureDriver(usize, DriverConfig),
  ActivateDriver(usize, ActivationMode, Option<usize>),
  DeactivateDriver(usize, DeactivationMode),
  RenderLedDeclarations(HashMap<u64, HashMap<&'static str, LedState>>),
  ConfigureSwitch(usize, bool, Option<Duration>, Option<Duration>),
}

// -- Events --

/// Runs when a switch becomes closed (depressed)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchClosed {
  pub switch: Switch,
}

impl SwitchClosed {
  pub fn new(switch: Switch) -> SwitchClosed {
    Self { switch }
  }
}

/// Runs when a switch becomes open (released)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchOpened {
  pub switch: Switch,
}

impl SwitchOpened {
  pub fn new(switch: Switch) -> SwitchOpened {
    Self { switch }
  }
}
