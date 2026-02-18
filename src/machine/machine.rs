use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use crate::machine::key_reader::monitor_keys;
use crate::machine::machine_config::MachineConfig;
use crate::machine::serial_interface::*;
use crate::machine::system_timer::{TimerMode, run_system_timers};
use crate::machine::watchdog::Watchdog;
use crate::prelude::*;
use crate::protocol::prelude::*;
use crate::protocol::*;
use crate::{hardware_definition::*, machine::machine_command::MachineCommand};
use crossterm::{
  event::{Event, KeyCode},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use tokio::sync::mpsc;

pub struct GameState {
  pub active_player: u8,
  pub player_count: u8,
}

pub struct Machine {
  io_port: SerialInterface,
  #[allow(unused)]
  exp_port: SerialInterface,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  driver_lookup: HashMap<&'static str, Driver>,
  watchdog: Watchdog,
  config: MachineConfig,
  expansion_boards: Vec<ExpansionBoardSpec>,
  system_tick: Duration,
  led_renderer: LedRenderer,

  runtime_stack: Vec<Box<dyn Runtime>>,
  switches: SwitchContext,
  game_state: Option<GameState>,

  command_sender: mpsc::UnboundedSender<MachineCommand>,
  command_receiver: mpsc::UnboundedReceiver<MachineCommand>,
}

impl Machine {
  pub(crate) fn new(
    io_port: SerialInterface,
    exp_port: SerialInterface,
    switches: SwitchContext,
    driver_lookup: HashMap<&'static str, Driver>,
    keyboard_switch_map: HashMap<KeyCode, usize>,
    config: MachineConfig,
    expansion_boards: Vec<ExpansionBoardSpec>,
  ) -> Self {
    let (command_sender, command_receiver) = mpsc::unbounded_channel();
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
      runtime_stack: Vec::new(),
      game_state: None,
      watchdog: Watchdog::new(
        Duration::from_millis(watchdog_interval),
        command_sender.clone(),
      ),
      command_sender,
      command_receiver,
      config,
      led_renderer: LedRenderer::new(&expansion_boards),
      expansion_boards,
      system_tick,
    }
  }

  pub async fn run(&mut self, runtime: Box<dyn Runtime>) {
    self.push_runtime(runtime);

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

    // initial startup for root scene
    self.dispatch_to_current_systems(|system, ctx| {
      system.on_system_enter(ctx);
    });

    log::info!("⟳ Machine run loop started.");

    loop {
      tokio::select! {
        Some(event) = self.io_port.read_event() => {
          // Add incoming hardware events to the command queue
          // this ensures they are processed in order with any commands emitted by systems in response to those events
          self.command_sender.send(MachineCommand::HardwareEvent(event)).ok();
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
    MachineBuilder::reset_expansion_boards(&mut self.exp_port, &self.expansion_boards).await;
  }

  async fn run_machine_command(&mut self, command: MachineCommand) {
    match command {
      MachineCommand::StartGame => self.start_game().await,
      MachineCommand::EndGame => self.end_game().await,
      MachineCommand::AddPlayer => self.add_player(),
      MachineCommand::AdvancePlayer => self.advance_player(),
      MachineCommand::PushRuntime(runtime_gen) => self.push_runtime(runtime_gen()),
      MachineCommand::PopRuntime => self.pop_runtime(),
      MachineCommand::PushScene(scene) => self.push_scene(scene),
      MachineCommand::PopScene => self.pop_scene(),
      MachineCommand::AddSystem(system) => self.add_system(system),
      MachineCommand::ReplaceSystem(system_id, system) => {
        self.replace_system(system_id, system);
      }
      MachineCommand::TerminateSystem(system_index) => self.terminate_system(system_index),
      MachineCommand::ConfigureDriver(driver_name, config) => {
        self.configure_driver(driver_name, config).await
      }
      MachineCommand::TriggerDriver(driver_name, mode, delay) => {
        self.trigger_driver(driver_name, mode, delay).await
      }
      MachineCommand::StoreWrite(f) => {
        let mut store = Store::new();
        f(&mut store);
      }
      MachineCommand::SetTimer(system_id, timer_name, duration, mode) => {
        self.set_system_timer(system_id, timer_name, duration, mode);
      }
      MachineCommand::ClearTimer(system_id, timer_name) => {
        self.clear_system_timer(system_id, timer_name);
      }
      MachineCommand::SetConfigValue(key, value) => {
        self.config.set_value(key, value);
      }
      MachineCommand::SystemTick => {
        let tick_duration = self.system_tick;
        self.dispatch_to_current_systems(|system, ctx| {
          system.on_tick(tick_duration, ctx);
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
      MachineCommand::Shutdown => {}
    }
  }

  // ---

  fn run_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    if let Some(switch) = self.switches.switch_by_id(&switch_id).cloned() {
      self.switches.update_switch_state(switch_id, state);
      let activated = matches!(state, SwitchState::Closed);

      self.dispatch_to_current_systems(|system, ctx| {
        if activated {
          system.on_switch_closed(&switch, ctx);
        } else {
          system.on_switch_opened(&switch, ctx);
        }
      });
    } else {
      log::warn!(
        "Received event for unknown switch ID {} : {:?}",
        switch_id,
        state
      );
      return;
    }
  }

  fn run_on_system_enter(&mut self) {
    self.dispatch_to_current_systems(|system, ctx| {
      system.on_system_enter(ctx);
    });
  }

  fn run_on_system_exit(&mut self) {
    self.dispatch_to_current_systems(|system, ctx| {
      system.on_system_exit(ctx);
    });
  }

  fn run_on_ball_start(&mut self) {
    self.dispatch_to_current_systems(|system, ctx| {
      system.on_ball_start(ctx);
    });
  }

  fn run_on_ball_end(&mut self) {
    self.dispatch_to_current_systems(|system, ctx| {
      system.on_ball_end(ctx);
    });
  }

  /// Run each system within the scene, capturing then running commands emitted during processing
  fn dispatch_to_current_systems<F>(&mut self, mut handler: F)
  where
    F: FnMut(&mut SystemContainer, &mut Context),
  {
    let runtime = self.runtime_stack.last_mut().unwrap();
    let (scene, store) = runtime.get_current_mut();

    log::debug!(
      "Dispatching to systems in scene, system count: {}",
      scene.len(),
    );

    for system in scene {
      let mut ctx = Context::new(
        self.command_sender.clone(),
        Some(store),
        &self.switches,
        &self.game_state,
        &self.config,
        Some(system.id),
      );
      handler(system, &mut ctx);
    }
  }

  pub(crate) async fn start_game(&mut self) {
    log::info!("Starting new game");
    self.game_state = Some(GameState {
      active_player: 0,
      player_count: 1,
    });
    self.enable_high_voltage().await;
    self.report_switches().await; // sync initial switch states
    self.run_on_system_enter();
  }

  async fn end_game(&mut self) {
    log::info!("Ending game");
    self.run_on_system_exit();
    self.disable_high_voltage().await;
    self.game_state = None;
  }

  fn add_player(&mut self) {
    log::info!("Adding player to game");
    if let Some(game_state) = &mut self.game_state {
      game_state.player_count += 1;
    } else {
      log::warn!("Attempted to add player but no game in progress");
    }
  }

  fn advance_player(&mut self) {
    log::info!("Advancing to next player");

    if self.game_state.is_none() {
      log::warn!("Attempted to advance player but no game in progress");
      return;
    }

    self.run_on_ball_end();
    if let Some(game_state) = &mut self.game_state {
      game_state.active_player += 1;
      if game_state.active_player >= game_state.player_count {
        game_state.active_player = 0;
      }
    }
    self.run_on_ball_start();
  }

  /// Transition to a new runtime
  pub fn push_runtime(&mut self, new_runtime: Box<dyn Runtime>) {
    log::info!("Pushing into new runtime");

    let mut ctx = Context::new(
      self.command_sender.clone(),
      None,
      &self.switches,
      &self.game_state,
      &self.config,
      None,
    );

    let runtime = self.runtime_stack.last_mut();
    if let Some(runtime) = runtime {
      log::trace!("on_runtime_exit for current runtime");
      runtime.on_runtime_exit(&mut ctx);
    }

    self.runtime_stack.push(new_runtime);

    log::trace!("on_runtime_enter for new runtime");
    self
      .runtime_stack
      .last_mut()
      .unwrap()
      .on_runtime_enter(&mut ctx);

    self.led_renderer.reset();
  }

  /// Transition out of current runtime back to previous
  pub fn pop_runtime(&mut self) {
    log::info!("Popping current runtime");
    let mut ctx = Context::new(
      self.command_sender.clone(),
      None,
      &self.switches,
      &self.game_state,
      &self.config,
      None,
    );

    let runtime = self.runtime_stack.last_mut();
    if let Some(runtime) = runtime {
      runtime.on_runtime_exit(&mut ctx);
    }

    self.runtime_stack.pop();

    if self.runtime_stack.len() > 0 {
      let runtime = self.runtime_stack.last_mut().unwrap();
      runtime.on_runtime_enter(&mut ctx);
    } else {
      log::warn!("No active runtime");
    }

    self.led_renderer.reset();
  }

  pub fn push_scene(&mut self, mut scene: Scene) {
    let runtime = self.runtime_stack.last_mut().unwrap();
    let store = runtime.get_current_store();

    for system in &mut scene {
      let mut ctx = Context::new(
        self.command_sender.clone(),
        Some(store),
        &self.switches,
        &self.game_state,
        &self.config,
        Some(system.id),
      );
      system.on_system_enter(&mut ctx);
    }

    runtime.push_scene(scene);
    self.led_renderer.reset();
  }

  pub fn pop_scene(&mut self) {
    let runtime = self.runtime_stack.last_mut().unwrap();
    let (outgoing_scene, store) = runtime.get_current_mut();

    for system in outgoing_scene {
      let mut ctx = Context::new(
        self.command_sender.clone(),
        Some(store),
        &self.switches,
        &self.game_state,
        &self.config,
        Some(system.id),
      );
      system.on_system_exit(&mut ctx);
    }

    runtime.pop_scene();
    self.led_renderer.reset();
  }

  pub fn add_system(&mut self, system: Box<dyn System>) {
    let runtime = self.runtime_stack.last_mut().unwrap();
    runtime
      .get_current_scene_mut()
      .push(SystemContainer::new(system));
  }

  /// Searches the current scene for the system, by ID
  fn find_system_index(&self, system_id: u64) -> Option<usize> {
    let runtime = self.runtime_stack.last().unwrap();
    let scene = runtime.get_current_scene();

    for (index, system) in scene.iter().enumerate() {
      if system.id == system_id {
        return Some(index);
      }
    }

    None
  }

  pub fn replace_system(&mut self, system_id: u64, new_system: Box<dyn System>) {
    let index = self.find_system_index(system_id);
    let runtime = self.runtime_stack.last_mut().unwrap();
    let scene = runtime.get_current_scene_mut();

    if let Some(index) = index {
      scene[index] = SystemContainer::new(new_system);
    } else {
      log::error!(
        "Attempted to replace system with invalid index: {}",
        system_id
      );
    }
  }

  pub fn terminate_system(&mut self, system_id: u64) {
    let index = self.find_system_index(system_id);
    let runtime = self.runtime_stack.last_mut().unwrap();
    let scene = runtime.get_current_scene_mut();

    if let Some(index) = index {
      scene.remove(index);
    } else {
      log::error!(
        "Attempted to terminate system with invalid index: {}",
        system_id
      );
    }
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

  fn set_system_timer(
    &mut self,
    system_id: u64,
    timer_name: &'static str,
    duration: Duration,
    mode: TimerMode,
  ) {
    let index = self.find_system_index(system_id);
    let runtime = self.runtime_stack.last_mut().unwrap();
    let scene = runtime.get_current_scene_mut();

    if let Some(system) = index.and_then(|index| scene.get_mut(index)) {
      system.set_timer(timer_name, duration, mode);
    } else {
      log::error!(
        "Attempted to set timer for invalid system index: {}",
        system_id
      );
    }
  }

  fn clear_system_timer(&mut self, system_id: u64, timer_name: &'static str) {
    let index = self.find_system_index(system_id);
    let runtime = self.runtime_stack.last_mut().unwrap();
    let scene = runtime.get_current_scene_mut();

    if let Some(system) = index.and_then(|index| scene.get_mut(index)) {
      system.clear_timer(timer_name);
    } else {
      log::error!(
        "Attempted to clear timer for invalid system index: {}",
        system_id
      );
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

  async fn render_leds(&mut self) {
    let runtime = self.runtime_stack.last_mut().unwrap();
    let scene = runtime.get_current_scene_mut();

    let mut declarations = HashMap::new();
    for system in scene {
      declarations.insert(system.id, system.leds(self.system_tick));
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
