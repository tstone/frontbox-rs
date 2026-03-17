use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use crate::hardware_definition::*;
use crate::machine::serial_interface::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::run_loop::SystemCollection;
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

pub struct Machine {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  led_renderer: LedRenderer,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
  watchdog_interval: Duration,
}

impl Machine {
  pub(crate) fn new(
    io_port: SerialInterface,
    exp_port: SerialInterface,
    led_renderer: LedRenderer,
  ) -> Self {
    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();
    let (app_sender, _) = mpsc::unbounded_channel::<AppMessage>(); // temporary

    Self {
      io_port,
      exp_port,
      app_sender, // this will overwritten set by App at startup
      machine_sender,
      machine_receiver,
      led_renderer,
      watchdog_interval: Duration::from_millis(1250),
    }
  }

  pub async fn read_io(&mut self) -> Option<EventResponse> {
    self.io_port.read_event().await
  }

  pub(crate) fn machine_sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  pub(crate) fn set_app_sender(&mut self, sender: mpsc::UnboundedSender<AppMessage>) {
    self.app_sender = sender;
  }

  pub async fn process_messages(&mut self) {
    while let Ok(cmd) = self.machine_receiver.try_recv() {
      match cmd {
        MachineMessage::WatchdogPing => {
          self.send_watchdog_ping().await;
        }
        MachineMessage::ClearWatchdog => {
          self.clear_watchdog().await;
        }
        MachineMessage::ResetExpansionNetwork(expansion_boards) => {
          self.reset_expansion_network(expansion_boards).await;
        }
        MachineMessage::ConfigureDriver(driver_id, config) => {
          self.configure_driver(driver_id, config).await;
        }
        MachineMessage::ActivateDriver(driver_id, mode, delay) => {
          self.activate_driver(driver_id, mode, delay).await;
        }
        MachineMessage::DeactivateDriver(driver_id, mode, delay) => {
          self.deactivate_driver(driver_id, mode, delay).await;
        }
        MachineMessage::ActivateDriverGroup(driver_ids, mode) => {
          for driver_id in driver_ids {
            self.activate_driver(driver_id, mode.clone(), None).await;
          }
        }
        MachineMessage::DeactivateDriverGroup(driver_ids, mode) => {
          for driver_id in driver_ids {
            self.deactivate_driver(driver_id, mode.clone(), None).await;
          }
        }
        MachineMessage::ReportSwitches => {
          self.report_switches().await;
        }
      }
    }
  }

  pub fn handle_switch_event(&mut self, switch_id: usize, state: SwitchState, ctx: &mut Context) {
    let switch_lookup = ctx.get_mut::<SwitchLookup>().unwrap();
    let switch = switch_lookup.switch_by_id(&switch_id).cloned();

    if let Some(switch) = switch {
      switch_lookup.update_switch_state(switch_id, state);

      if matches!(state, SwitchState::Closed) {
        ctx.emit(SwitchClosed::new(switch));
      } else {
        ctx.emit(SwitchOpened::new(switch));
      }
    } else {
      // Report as native board/switch id since this is the easiest way to figure out current switch wiring
      match Self::get_native_switch_id(switch_id, ctx) {
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

  pub async fn render_leds(
    &mut self,
    sc: &mut SystemCollection,
    tick_interval: Duration,
    ctx_template: &mut Context<'_>,
  ) {
    let mut declarations = HashMap::new();

    // gather LED declarations from all active systems and child systems (within)
    for system in sc.systems.values_mut() {
      let ctx = ctx_template.clone_for_system(system.id);
      if system.is_active(&ctx) {
        declarations.insert(system.id, system.leds(tick_interval, &ctx));
      }
    }
    for group in sc.groups.values_mut() {
      for system in group.systems.values_mut() {
        let ctx = ctx_template.clone_for_system(system.id);
        if system.is_active(&ctx) {
          declarations.insert(system.id, system.leds(tick_interval, &ctx));
        }
      }
    }

    self.led_renderer.tick(tick_interval);
    self
      .led_renderer
      .render(&mut self.exp_port, declarations)
      .await;
  }

  // ---

  pub async fn send_watchdog_ping(&mut self) {
    let _ = self
      .io_port
      .request(
        &WatchdogCommand::set(self.watchdog_interval),
        Duration::from_secs(1),
      )
      .await;
  }

  pub async fn clear_watchdog(&mut self) {
    let _ = self
      .io_port
      .request(&WatchdogCommand::disable(), Duration::from_secs(1))
      .await;
  }

  /// Primarily used for reporting of unkown switches as native board/switch ids
  fn get_native_switch_id(switch_id: usize, ctx: &Context) -> Option<(usize, usize)> {
    let mut offset: usize = 0;
    let io_boards = ctx.get::<IoBoards>().unwrap();
    for (index, board) in io_boards.iter().enumerate() {
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
        Duration::from_secs(2),
      )
      .await
    {
      Ok(ProcessedResponse::Processed) => {
        log::debug!("Driver {} configured successfully", driver);
      }
      Ok(ProcessedResponse::Failed) => {
        log::error!("Driver {} configuration failed", driver);
      }
      Err(e) => {
        log::error!("Error configuring driver {}: {}", driver, e);
      }
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
    delay: Option<Duration>,
  ) {
    log::info!("Activating driver {} with mode {:?}", driver, mode);
    let control_mode: DriverTriggerControlMode = match mode {
      ActivationMode::Automatic => DriverTriggerControlMode::Automatic,
      ActivationMode::Tap => DriverTriggerControlMode::Manual,
      ActivationMode::VirtualSwitchOn => DriverTriggerControlMode::On,
    };
    self.trigger_driver(driver, control_mode, delay).await;
  }

  pub async fn deactivate_driver(
    &mut self,
    driver: usize,
    mode: DeactivationMode,
    delay: Option<Duration>,
  ) {
    log::info!("Deactivating driver {} with mode {:?}", driver, mode);
    let control_mode: DriverTriggerControlMode = match mode {
      DeactivationMode::Disabled => DriverTriggerControlMode::Automatic,
      DeactivationMode::VirtualSwitchOff => DriverTriggerControlMode::Off,
    };
    self.trigger_driver(driver, control_mode, delay).await;
  }

  async fn trigger_driver(
    &mut self,
    driver: usize,
    mode: DriverTriggerControlMode,
    delay: Option<Duration>,
  ) {
    if let Some(delay) = delay {
      tokio::time::sleep(delay).await;
    }

    log::info!("Triggering driver {}", driver);
    self
      .io_port
      .dispatch(&TriggerDriverCommand::new(driver, mode, None))
      .await;
  }

  pub async fn reset_expansion_network(&mut self, expansion_boards: ExpansionBoards) {
    self.led_renderer.reset();

    for board in expansion_boards.iter() {
      // TODO: move this to a better common location
      App::reset_expansion_board(&mut self.exp_port, board).await;
    }
  }
}

/// While Machine can *technically* be used directly as a System, this creates problems when the App needs to query for
/// read the I/O hardware events. Instead of dealing with some kind of Arc reference, the bridge allows the Machine to be
/// owned by the App while still exposing commands to interact with it as a System.
pub(crate) struct MachineBridge {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
}

impl MachineBridge {
  pub(crate) fn new(machine_sender: mpsc::UnboundedSender<MachineMessage>) -> Box<Self> {
    Box::new(Self { machine_sender })
  }
}

impl System for MachineBridge {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<WatchdogPing>();
    ctx.register_command::<ClearWatchdog>();
    ctx.register_command::<ResetExpansionNetwork>();
    ctx.register_command::<ConfigureDriver>();
    ctx.register_command::<ActivateDriver>();
    ctx.register_command::<ActivateDriverDelayed>();
    ctx.register_command::<DeactivateDriver>();
    ctx.register_command::<DeactivateDriverDelayed>();
    ctx.register_command::<ActivateDriverGroup>();
    ctx.register_command::<DeactivateDriverGroup>();
    ctx.register_command::<RefreshSwitchState>();
  }

  fn on_command(&mut self, cmd: &dyn Command, ctx: &mut Context) {
    if let Some(_) = cmd.as_any().downcast_ref::<WatchdogPing>() {
      self.machine_sender.send(MachineMessage::WatchdogPing).ok();
    } else if let Some(_) = cmd.as_any().downcast_ref::<ClearWatchdog>() {
      self.machine_sender.send(MachineMessage::ClearWatchdog).ok();
    } else if let Some(_) = cmd.as_any().downcast_ref::<ResetExpansionNetwork>() {
      let boards = ctx.cloned::<ExpansionBoards>().unwrap();
      self
        .machine_sender
        .send(MachineMessage::ResetExpansionNetwork(boards))
        .ok();
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<ConfigureDriver>() {
      let driver_lookup = ctx.expect::<DriverLookup>();
      if let Some(driver) = driver_lookup.get(cmd.driver) {
        let switch_lookup = ctx.expect::<SwitchLookup>();
        let config = cmd.mode.to_config(switch_lookup);
        self
          .machine_sender
          .send(MachineMessage::ConfigureDriver(driver.id, config))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<ActivateDriver>() {
      let driver_lookup = ctx.expect::<DriverLookup>();
      if let Some(driver) = driver_lookup.get(cmd.driver) {
        self
          .machine_sender
          .send(MachineMessage::ActivateDriver(
            driver.id,
            cmd.mode.clone(),
            None,
          ))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<ActivateDriverDelayed>() {
      let driver_lookup = ctx.expect::<DriverLookup>();
      if let Some(driver) = driver_lookup.get(cmd.driver) {
        self
          .machine_sender
          .send(MachineMessage::ActivateDriver(
            driver.id,
            cmd.mode.clone(),
            Some(cmd.delay),
          ))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<DeactivateDriver>() {
      let driver_lookup = ctx.expect::<DriverLookup>();
      if let Some(driver) = driver_lookup.get(cmd.driver) {
        self
          .machine_sender
          .send(MachineMessage::DeactivateDriver(
            driver.id,
            cmd.mode.clone(),
            None,
          ))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<DeactivateDriverDelayed>() {
      let driver_lookup = ctx.expect::<DriverLookup>();
      if let Some(driver) = driver_lookup.get(cmd.driver) {
        self
          .machine_sender
          .send(MachineMessage::DeactivateDriver(
            driver.id,
            cmd.mode.clone(),
            Some(cmd.delay),
          ))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<ActivateDriverGroup>() {
      let lookup = ctx.expect::<DriverLookup>();
      let groups = ctx.expect::<DriverGroups>();
      if let Some(group) = groups.get(cmd.group) {
        let driver_ids: Vec<usize> = group
          .iter()
          .filter_map(|name| lookup.get(name))
          .map(|driver| driver.id)
          .collect();
        self
          .machine_sender
          .send(MachineMessage::ActivateDriverGroup(
            driver_ids,
            cmd.mode.clone(),
          ))
          .ok();
      }
    } else if let Some(cmd) = cmd.as_any().downcast_ref::<DeactivateDriverGroup>() {
      let lookup = ctx.expect::<DriverLookup>();
      let groups = ctx.expect::<DriverGroups>();
      if let Some(group) = groups.get(cmd.group) {
        let driver_ids: Vec<usize> = group
          .iter()
          .filter_map(|name| lookup.get(name))
          .map(|driver| driver.id)
          .collect();
        self
          .machine_sender
          .send(MachineMessage::DeactivateDriverGroup(
            driver_ids,
            cmd.mode.clone(),
          ))
          .ok();
      }
    } else if let Some(_) = cmd.as_any().downcast_ref::<RefreshSwitchState>() {
      self
        .machine_sender
        .send(MachineMessage::ReportSwitches)
        .ok();
    }
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    let boards = ctx.expect::<ExpansionBoards>().clone();
    self
      .machine_sender
      .send(MachineMessage::ResetExpansionNetwork(boards))
      .ok();
  }
}

#[derive(Debug)]
pub(crate) enum MachineMessage {
  WatchdogPing,
  ClearWatchdog,
  ReportSwitches,
  ResetExpansionNetwork(ExpansionBoards),
  ConfigureDriver(usize, DriverConfig),
  ActivateDriver(usize, ActivationMode, Option<Duration>),
  DeactivateDriver(usize, DeactivationMode, Option<Duration>),
  ActivateDriverGroup(Vec<usize>, ActivationMode),
  DeactivateDriverGroup(Vec<usize>, DeactivationMode),
}

// -- Events --

/// Runs when a switch becomes closed (depressed)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchClosed {
  pub switch: Switch,
}

impl SwitchClosed {
  pub fn new(switch: Switch) -> Box<SwitchClosed> {
    Box::new(Self { switch })
  }
}

/// Runs when a switch becomes open (released)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchOpened {
  pub switch: Switch,
}

impl SwitchOpened {
  pub fn new(switch: Switch) -> Box<SwitchOpened> {
    Box::new(Self { switch })
  }
}
