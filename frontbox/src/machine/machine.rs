use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use crate::machine::event::FrontboxEvent;
use crate::machine::event::*;
use crate::machine::key_reader::monitor_keys;
use crate::machine::machine_config::MachineConfig;
use crate::machine::serial_interface::*;
use crate::machine::watchdog::Watchdog;
use crate::prelude::*;
use crate::systems::SystemCommand;
use crate::systems::SystemCommands;
use crate::systems::{SystemContainer, run_system_timers};
use crate::{hardware_definition::*, machine::machine_command::MachineCommand};
use crossterm::{
  event::{Event, KeyCode},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use fast_protocol::*;
use tokio::sync::mpsc;

pub struct GameState {
  pub active_player: u8,
  pub player_count: u8,
}

pub struct Machine {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  driver_lookup: HashMap<&'static str, DriverDefinition>,
  watchdog: Watchdog,
  config: MachineConfig,
  io_boards: Vec<IoBoardDefinition>,
  expansion_boards: Vec<ExpansionBoardDefinition>,
  system_tick: Duration,
  led_renderer: LedRenderer,
  global_store: Store,
  global_systems: Vec<SystemContainer>,
  switches: SwitchContext,
  game_state: Option<GameState>,
  states: States,
  command_sender: mpsc::UnboundedSender<MachineCommand>,
  command_receiver: mpsc::UnboundedReceiver<MachineCommand>,
  system_sender: mpsc::UnboundedSender<SystemCommand>,
  system_receiver: mpsc::UnboundedReceiver<SystemCommand>,
  store_sender: mpsc::UnboundedSender<StoreCommand>,
  store_receiver: mpsc::UnboundedReceiver<StoreCommand>,
}

impl Machine {
  pub(crate) fn new(
    io_port: SerialInterface,
    exp_port: SerialInterface,
    switches: SwitchContext,
    driver_lookup: HashMap<&'static str, DriverDefinition>,
    keyboard_switch_map: HashMap<KeyCode, usize>,
    config: MachineConfig,
    io_boards: Vec<IoBoardDefinition>,
    expansion_boards: Vec<ExpansionBoardDefinition>,
  ) -> Self {
    let (command_sender, command_receiver) = mpsc::unbounded_channel();
    let (system_sender, system_receiver) = mpsc::unbounded_channel();
    let (store_sender, store_receiver) = mpsc::unbounded_channel();
    let watchdog_interval = config
      .get_value_as_u64(default_config::WATCHDOG_TICK)
      .unwrap_or(1000);

    let system_tick = Duration::from_millis(
      config
        .get_value_as_u64(default_config::SYSTEM_TIMER_TICK)
        .unwrap(),
    );

    Self {
      io_port,
      exp_port,
      switches: switches,
      driver_lookup,
      keyboard_switch_map,
      game_state: None,
      watchdog: Watchdog::new(
        Duration::from_millis(watchdog_interval),
        command_sender.clone(),
      ),
      command_sender,
      command_receiver,
      system_sender,
      system_receiver,
      store_sender,
      store_receiver,
      config,
      led_renderer: LedRenderer::new(&expansion_boards),
      io_boards,
      expansion_boards,
      system_tick,
      global_store: Store::new(),
      global_systems: Vec::new(),
      states: States::new(),
    }
  }

  pub async fn run(&mut self, systems: Vec<Box<dyn System>>) {
    // initialize systems
    {
      let ctx = Context::new(
        &self.config,
        &self.game_state,
        &self.states,
        &self.global_store,
        &self.switches,
      );
      for system in systems.into_iter() {
        let mut cmds = Commands::new(
          self.command_sender.clone(),
          self.system_sender.clone(),
          self.store_sender.clone(),
          0,
        );
        SystemCommands::spawn_system(system, &mut self.global_systems, &ctx, &mut cmds);
      }
    }

    // initialize keyboard monitoring if there are any keyboard-mapped switches
    if self.keyboard_switch_map.len() > 0 {
      match enable_raw_mode() {
        Ok(_) => {}
        Err(e) => {
          log::error!("Failed to enable raw mode for keyboard input: {}", e);
        }
      }
      monitor_keys(self.command_sender.clone());
    }

    // system tick manages the timers within systems
    run_system_timers(self.system_tick.clone(), self.command_sender.clone());

    // listen for ctrl-c to trigger shutdown
    let tx = self.command_sender.clone();
    tokio::spawn(async move {
      tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
      let _ = tx.send(MachineCommand::Shutdown);
    });

    log::info!("⟳ Machine run loop started.");

    loop {
      tokio::select! {
        Some(event) = self.io_port.read_event() => {
          // Add incoming hardware events to the command queue
          // this ensures they are processed in order with any commands emitted by systems in response to those events
          self.command_sender.send(MachineCommand::HardwareEvent(event)).ok();
        }

        Some(command) = self.system_receiver.recv() => {
          let ctx =     Context::new(
            &self.config,
            &self.game_state,
            &self.states,
            &self.global_store,
            &self.switches,
          );
          let mut cmds = Commands::new(
            self.command_sender.clone(),
            self.system_sender.clone(),
            self.store_sender.clone(),
            0,
          );
          SystemCommands::process(command, &mut self.global_systems, &ctx, &mut cmds);
        }

        Some(command) = self.store_receiver.recv() => {
          match command {
            StoreCommand::Write(f) => {
              f(&mut self.global_store);
            }
          }
        }

        Some(command) = self.command_receiver.recv() => {
          if matches!(command, MachineCommand::SystemTick)
            || matches!(command, MachineCommand::WatchdogTick)
          {
            log::trace!("Executing machine command: {:?}", command);
          } else {
            log::debug!("Executing machine command: {:?}", command);
          }

          if matches!(command, MachineCommand::Shutdown) {
            log::info!("Shutdown command received, exiting machine run loop.");
            break;
          }

          self.run_machine_command(command).await;
        }
      }
    }

    if self.keyboard_switch_map.len() > 0 {
      disable_raw_mode().ok();
    }

    self
      .io_port
      .request(&WatchdogCommand::disable(), Duration::from_secs(2))
      .await
      .ok();

    // Reset expansion boards (LEDs servos, etc.) to an off/default state
    self.reset_expansion_network().await;
  }

  async fn run_machine_command(&mut self, command: MachineCommand) {
    match command {
      MachineCommand::StartGame => self.start_game().await,
      MachineCommand::EndGame => self.end_game().await,
      MachineCommand::AddPlayer => self.add_player(),
      MachineCommand::AdvancePlayer => self.advance_player().await,
      MachineCommand::ConfigureDriver(driver_name, config) => {
        self.configure_driver(driver_name, config).await
      }
      MachineCommand::TriggerDriver(driver_name, mode, delay) => {
        self.trigger_driver(driver_name, mode, delay).await
      }
      MachineCommand::SetConfigValue(key, value) => {
        self.config.set_value(key, value);
      }
      MachineCommand::SystemTick => {
        let tick_duration = self.system_tick;
        self.dispatch_to_current_systems(|system, ctx, cmds| {
          system.on_tick(tick_duration, ctx, cmds);
        });
        self.render_leds().await;
      }
      MachineCommand::HardwareEvent(event) => match event {
        EventResponse::Switch { switch_id, state } => self.run_switch_event(switch_id, state),
      },
      MachineCommand::Key(event) => self.on_key_press(event),
      MachineCommand::WatchdogTick => {
        let _ = self
          .io_port
          .request(
            &WatchdogCommand::set(Duration::from_millis(1250)),
            Duration::from_secs(1),
          )
          .await;
      }
      MachineCommand::ResetExpansionNetwork => {
        self.reset_expansion_network().await;
      }
      MachineCommand::Shutdown => {}
      MachineCommand::EmitEvent(e) => self.emit(e),
      MachineCommand::StateTransition(f) => f(&mut self.states),
    }
  }

  // ---

  fn emit(&mut self, event: Box<dyn FrontboxEvent>) {
    self.dispatch_to_current_systems(|system, ctx, cmds| {
      system.on_event(event.as_ref(), ctx, cmds);
    });
  }

  fn run_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    if let Some(switch) = self.switches.switch_by_id(&switch_id).cloned() {
      self.switches.update_switch_state(switch_id, state);

      if matches!(state, SwitchState::Closed) {
        self.emit(SwitchClosed::new(switch));
      } else {
        self.emit(SwitchOpened::new(switch));
      }
    } else {
      // Repor as native board/switch id since this is the easiest way to figure out current switch wiring
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

  fn get_native_switch_id(&self, switch_id: usize) -> Option<(usize, usize)> {
    let mut offset: usize = 0;
    for (index, board) in self.io_boards.iter().enumerate() {
      if switch_id < (board.switch_count as usize) + offset {
        let native_switch_id = switch_id - offset;
        return Some((index, native_switch_id));
      }
      offset += board.switch_count as usize;
    }
    None
  }

  /// Run each root system
  fn dispatch_to_current_systems<F>(&mut self, mut handler: F)
  where
    F: FnMut(&mut SystemContainer, &Context, &mut Commands),
  {
    let ctx = Context::new(
      &self.config,
      &self.game_state,
      &self.states,
      &self.global_store,
      &self.switches,
    );

    for system in self.global_systems.iter_mut() {
      let mut cmds = Commands::new(
        self.command_sender.clone(),
        self.system_sender.clone(),
        self.store_sender.clone(),
        system.id,
      );
      if system.is_active(&ctx) {
        handler(system, &ctx, &mut cmds);
      }
    }
  }

  pub(crate) async fn start_game(&mut self) {
    if self.game_state.is_some() {
      return;
    }

    log::info!("Starting new game");
    self.game_state = Some(GameState {
      active_player: 0,
      player_count: 1,
    });
    self.enable_high_voltage().await;
    self.report_switches().await; // sync initial switch states
    self.emit(GameStarted::new());
  }

  async fn end_game(&mut self) {
    log::info!("Ending game");
    self.emit(GameEnded::new());
    self.disable_high_voltage().await;
    self.game_state = None;
  }

  fn add_player(&mut self) {
    log::info!("Adding player to game");
    if let Some(game_state) = &mut self.game_state {
      game_state.player_count += 1;
      let player_count = game_state.player_count;
      self.emit(PlayerAdded::new(player_count));
    } else {
      log::warn!("Attempted to add player but no game in progress");
    }
  }

  async fn advance_player(&mut self) {
    log::info!("Advancing to next player");

    if self.game_state.is_none() {
      log::warn!("Attempted to advance player but no game in progress");
      return;
    }

    if let Some(game_state) = &mut self.game_state {
      game_state.active_player += 1;
      if game_state.active_player >= game_state.player_count {
        game_state.active_player = 0;
      }
    }

    self.reset_expansion_network().await;
    self.report_switches().await;
  }

  async fn enable_high_voltage(&mut self) {
    log::info!("Enabling high voltage");
    self.watchdog.enable();
    let _ = self
      .io_port
      .request(
        &WatchdogCommand::set(Duration::from_millis(1250)),
        Duration::from_secs(1),
      )
      .await;

    // give some time for the hardware to power up before we start sending commands
    tokio::time::sleep(Duration::from_millis(300)).await;
  }

  async fn disable_high_voltage(&mut self) {
    log::info!("Disabling high voltage");
    self.watchdog.disable();

    // Clear any remaining watchdog time out
    let _ = self
      .io_port
      .request(&WatchdogCommand::disable(), Duration::from_secs(1))
      .await;
  }

  async fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
    match self.driver_lookup.get(driver) {
      Some(driver) => {
        log::info!("Configuring driver {}", driver.name);
        match self
          .io_port
          .request(
            &ConfigureDriverCommand::new(&driver.id, &config),
            Duration::from_secs(2),
          )
          .await
        {
          Ok(ProcessedResponse::Processed) => {
            log::debug!("Driver {} configured successfully", driver.name);
          }
          Ok(ProcessedResponse::Failed) => {
            log::error!("Driver {} configuration failed", driver.name);
          }
          Err(e) => {
            log::error!("Error configuring driver {}: {}", driver.name, e);
          }
        }
      }
      None => {
        log::error!("Attempted to configure unknown driver: {}", driver);
        return;
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
        self.switches.update_switch_states(switches);
      }
      _ => {
        log::error!("Failed to report switches");
      }
    }
  }

  async fn trigger_driver(
    &mut self,
    driver: &'static str,
    mode: DriverTriggerControlMode,
    delay: Option<Duration>,
  ) {
    match self.driver_lookup.get(driver) {
      Some(driver) => {
        if let Some(delay) = delay {
          tokio::time::sleep(delay).await;
        }

        log::info!("Triggering driver {}", driver.name);
        let switch = driver.config.as_ref().and_then(|cfg| cfg.switch_id());
        self
          .io_port
          .dispatch(&TriggerDriverCommand::new(driver.id, mode, switch))
          .await;
      }
      None => {
        log::error!("Attempted to trigger unknown driver: {}", driver);
        return;
      }
    }
  }

  fn on_key_press(&mut self, event: Event) {
    match event {
      Event::Key(key) => {
        if let Some(&switch_id) = self.keyboard_switch_map.get(&key.code) {
          let state = if key.kind == crossterm::event::KeyEventKind::Release {
            SwitchState::Open
          } else {
            SwitchState::Closed
          };
          log::debug!(
            "Keyboard event: {:?}, triggering switch ID {} to {:?}",
            key,
            switch_id,
            state
          );
          self.run_switch_event(switch_id, state);
        }
      }
      _ => {}
    }
  }

  async fn reset_expansion_network(&mut self) {
    self.led_renderer.reset();
    // TODO: move this to a better common location
    MachineBuilder::reset_expansion_boards(&mut self.exp_port, &self.expansion_boards).await;
  }

  async fn render_leds(&mut self) {
    let ctx = Context::new(
      &self.config,
      &self.game_state,
      &self.states,
      &self.global_store,
      &self.switches,
    );

    let mut declarations = HashMap::new();
    for system in self.global_systems.iter_mut() {
      declarations.insert(system.id, system.leds(self.system_tick, &ctx));
    }

    self.led_renderer.tick(self.system_tick);
    self
      .led_renderer
      .render(&mut self.exp_port, declarations)
      .await;
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}

impl Switch {
  pub fn is_virtual(&self) -> bool {
    self.id > u16::MAX as usize
  }
}
