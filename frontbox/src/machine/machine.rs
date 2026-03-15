use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use crate::hardware_definition::*;
use crate::machine::serial_interface::*;
use crate::prelude::*;
use crate::systems::SystemContainer;
use fast_protocol::*;
use tokio::sync::mpsc;

#[derive(Debug, Default, Serialize, Storable)]
pub struct GameState {
  pub active_player: u8,
  pub player_count: u8,
}

pub struct Machine {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  led_renderer: LedRenderer,
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

    Self {
      io_port,
      exp_port,
      // TODO:
      // watchdog: Watchdog::new(watchdog_tick, system_sender.clone()),
      // watchdog_tick,
      machine_sender,
      machine_receiver,
      led_renderer,
      watchdog_interval: Duration::from_millis(1250),
    }
  }

  pub async fn read_io(&mut self) -> Option<EventResponse> {
    self.io_port.read_event().await
  }

  pub async fn listen(&mut self) {
    loop {
      tokio::select! {
        Some(cmd) = self.machine_receiver.recv() => {
          match cmd {
            MachineMessage::WatchdogPing => {
              self.send_watchdog_ping().await;
            }
            MachineMessage::ClearWatchdog => {
              self.clear_watchdog().await;
            }
            MachineMessage::ResetExpansionNetwork(expansion_boards) => {
              self.reset_expansion_network(expansion_boards);
            }
          }
        }
      }
    }
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

  /// Primarily used for reporting of unkown switches as native board/switch ids, but can also be used for virtual switches
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

  // TODO: these game/player commands don't need to be on machine itself

  // pub(crate) async fn start_game(&mut self, store: &mut Store) {
  //   if self.game_state.is_some() {
  //     return;
  //   }

  //   log::info!("Starting new game");
  //   self.game_state = Some(GameState {
  //     active_player: 0,
  //     player_count: 1,
  //   });
  //   self.report_switches().await; // sync initial switch states
  //   self.emit(GameStarted::new(), store);
  // }

  // async fn end_game(&mut self, store: &mut Store) {
  //   log::info!("Ending game");
  //   self.emit(GameEnded::new(), store);
  //   self.game_state = None;
  // }

  // fn add_player(&mut self, store: &mut Store) {
  //   log::info!("Adding player to game");
  //   if let Some(game_state) = &mut self.game_state {
  //     game_state.player_count += 1;
  //     let player_count = game_state.player_count;
  //     self.emit(PlayerAdded::new(player_count), store);
  //   } else {
  //     log::warn!("Attempted to add player but no game in progress");
  //   }
  // }

  // async fn advance_player(&mut self, store: &mut Store) {
  //   log::info!("Advancing to next player");

  //   let game_state = store.get_mut::<GameState>();

  //   if game_state.is_none() {
  //     log::warn!("Attempted to advance player but no game in progress");
  //     return;
  //   }

  //   if let Some(game_state) = game_state {
  //     game_state.active_player += 1;
  //     if game_state.active_player >= game_state.player_count {
  //       game_state.active_player = 0;
  //     }
  //   }

  //   self.reset_expansion_network().await;
  //   self.report_switches().await;
  // }

  async fn configure_driver(
    &mut self,
    driver: &'static str,
    mode: Box<dyn DriverMode>,
    store: &Store,
  ) {
    let driver_lookup = store.get::<DriverLookup>().unwrap();
    match driver_lookup.get(driver) {
      Some(driver) => {
        let switch_lookup = store.get::<SwitchLookup>().unwrap();
        let config = mode.to_config(switch_lookup);

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

  async fn report_switches(&mut self, store: &mut Store) {
    match self
      .io_port
      .request(&ReportSwitchesCommand::new(), Duration::from_secs(2))
      .await
    {
      Ok(SwitchReportResponse::SwitchReport { switches }) => {
        let switch_lookup = store.get_mut::<SwitchLookup>().unwrap();
        switch_lookup.update_switch_states(switches);
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
    store: &Store,
  ) {
    let driver_lookup = store.get::<DriverLookup>().unwrap();
    match driver_lookup.get(driver) {
      Some(driver) => {
        if let Some(delay) = delay {
          tokio::time::sleep(delay).await;
        }

        log::info!("Triggering driver {}", driver.name);
        self
          .io_port
          .dispatch(&TriggerDriverCommand::new(driver.id, mode, None))
          .await;
      }
      None => {
        log::error!("Attempted to trigger unknown driver: {}", driver);
        return;
      }
    }
  }

  async fn trigger_driver_group(
    &mut self,
    group_name: &'static str,
    mode: DriverTriggerControlMode,
    store: &Store,
  ) {
    let Some(group) = store
      .get::<DriverGroups>()
      .and_then(|groups| groups.get(group_name))
    else {
      log::error!("Attempted to trigger unknown driver group: {}", group_name);
      return;
    };

    let drivers: Vec<_> = group.iter().cloned().collect();
    for driver_name in drivers {
      self.trigger_driver(driver_name, mode, None, store).await;
    }
  }

  pub async fn reset_expansion_network(&mut self, expansion_boards: ExpansionBoards) {
    self.led_renderer.reset();

    for board in expansion_boards.iter() {
      // TODO: move this to a better common location
      App::reset_expansion_board(&mut self.exp_port, board).await;
    }
  }

  pub async fn render_leds(
    &mut self,
    systems: &mut Vec<SystemContainer>,
    tick_interval: Duration,
    ctx_template: &mut Context<'_>,
  ) {
    let mut declarations = HashMap::new();
    for system in systems.iter_mut() {
      let ctx = ctx_template.clone_for_system(system.id);
      declarations.insert(system.id, system.leds(tick_interval, &ctx));
    }

    self.led_renderer.tick(tick_interval);
    self
      .led_renderer
      .render(&mut self.exp_port, declarations)
      .await;
  }
}

/// A bridge between Machine and systems
struct MachineSystem {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
}

impl System for MachineSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    let sender = self.machine_sender.clone();
    ctx.register_command::<WatchdogPing>(move |_, _ctx| {
      sender.send(MachineMessage::WatchdogPing);
    });

    let sender = self.machine_sender.clone();
    ctx.register_command::<ClearWatchdog>(move |_, _ctx| {
      sender.send(MachineMessage::ClearWatchdog);
    });

    let sender = self.machine_sender.clone();
    ctx.register_command::<ResetExpansionNetwork>(move |_, ctx| {
      let boards = ctx.cloned::<ExpansionBoards>().unwrap();
      sender.send(MachineMessage::ResetExpansionNetwork(boards));
    });
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    let boards = ctx.expect::<ExpansionBoards>().clone();
    self
      .machine_sender
      .send(MachineMessage::ResetExpansionNetwork(boards));
  }
}

enum MachineMessage {
  WatchdogPing,
  ClearWatchdog,
  ResetExpansionNetwork(ExpansionBoards),
}

// -- Commands --
pub struct WatchdogPing;
pub struct ClearWatchdog;
pub struct ResetExpansionNetwork;

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
