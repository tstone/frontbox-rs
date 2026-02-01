use std::collections::HashMap;
use std::fmt::Debug;

use crate::hardware::driver_config::DriverConfig;
use crate::mainboard::{MainboardCommand, MainboardIncoming};
use crate::modes::game_context::GameCommand;
use crate::modes::machine_context::MachineCommand;
use crate::modes::prelude::*;
use crate::protocol::{self, FastResponse, SwitchState};
use crate::store::Store;
use crate::switch_context::SwitchContext;
use crate::{DriverPin, IoNetwork, Mainboard, prelude::*};
use crossterm::{
  event::{Event, EventStream, KeyCode},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub type MachineFrame = Vec<Box<dyn MachineMode>>;
pub type GameFrame = Vec<Box<dyn GameMode>>;

#[derive(Debug)]
pub struct Machine {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: SwitchContext,
  driver_lookup: HashMap<&'static str, DriverPin>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  machine_stack: Vec<MachineFrame>,
  init_game_stack: Vec<GameFrame>,
  current_game_stack: Vec<Vec<GameFrame>>,
  machine_store: Store,
  player_stores: Vec<Store>,
  player_points: Vec<u32>,
  game: GameState,
}

impl Machine {
  pub async fn boot(config: BootConfig, io_network: IoNetwork) -> Self {
    let (command_tx, command_rx) = mpsc::channel::<MainboardCommand>(128);
    let (event_tx, event_rx) = mpsc::channel::<MainboardIncoming>(128);

    let mut mainboard = Mainboard::boot(config, command_rx, event_tx.clone()).await;

    // start serial communication in separate thread
    std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

      runtime.block_on(async move {
        mainboard.run().await;
      });
    });

    // TODO: define LEDs
    // TODO: read state of all switches from mainboard and setup starting state
    let switches = SwitchContext::new(io_network.switches);

    let mut drivers = HashMap::new();
    for driver in io_network.driver_pins {
      drivers.insert(driver.name, driver);
    }

    Self {
      command_tx,
      event_rx,
      switches,
      driver_lookup: drivers,
      keyboard_switch_map: HashMap::new(),
      machine_stack: Vec::new(),
      player_stores: Vec::new(),
      machine_store: Store::new(),
      game: GameState::default(),
      init_game_stack: Vec::new(),
      current_game_stack: Vec::new(),
      player_points: Vec::new(),
    }
  }

  pub fn add_machine_frame(&mut self, frame: MachineFrame) -> &mut Self {
    self.machine_stack.push(frame);
    self
  }

  pub fn add_game_frame(&mut self, frame: GameFrame) -> &mut Self {
    self.init_game_stack.push(frame);
    self
  }

  pub fn add_keyboard_mapping(&mut self, key: KeyCode, switch_name: &'static str) -> &mut Self {
    let switch = self.switches.switch_by_name(switch_name).unwrap();
    self.keyboard_switch_map.insert(key, switch.id);
    self
  }

  pub fn add_keyboard_mappings(&mut self, mappings: Vec<(KeyCode, &'static str)>) -> &mut Self {
    for (key, switch_name) in mappings {
      self.add_keyboard_mapping(key, switch_name);
    }
    self
  }

  pub async fn run(&mut self) {
    if self.keyboard_switch_map.len() > 0 {
      match enable_raw_mode() {
        Ok(_) => {}
        Err(e) => {
          log::error!("Failed to enable raw mode for keyboard input: {}", e);
        }
      }
    }

    let mut key_reader = EventStream::new();

    loop {
      tokio::select! {
        Some(event) = self.event_rx.recv() => {
          match event.data {
            FastResponse::Switch { switch_id, state } => self.run_switch_event(switch_id, state),
            _ => {
              // handle other events
            }
          }
        }

        Some(Ok(event)) = key_reader.next(), if self.keyboard_switch_map.len() > 0 => {
          match event {
            Event::Key(key) => {
              if key.code == KeyCode::Esc || (key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)) {
                break;
              }

              if let Some(&switch_id) = self.keyboard_switch_map.get(&key.code) {
                let state = if key.kind == crossterm::event::KeyEventKind::Release {
                  SwitchState::Open
                } else {
                  SwitchState::Closed
                };
                log::debug!("Keyboard event: {:?}, triggering switch ID {} to {:?}", key, switch_id, state);
                self.run_switch_event(switch_id, state);
              }
            }
            _ => {}
          }
        }
      }
    }

    if self.keyboard_switch_map.len() > 0 {
      let _ = disable_raw_mode();
    }
  }

  fn run_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    if let Some(switch) = self.switches.switch_by_id(&switch_id).cloned() {
      self.switches.update_switch_state(switch_id, state);

      let mut machine_commands = Vec::new();
      let mut game_commands = Vec::new();
      let activated = matches!(state, SwitchState::Closed);

      // Machine stack
      let current_machine_frame = self.machine_stack.last_mut().unwrap();
      for mode in current_machine_frame {
        if mode.is_listening() {
          let mut ctx = MachineContext::new(&self.game, &mut self.machine_store, &self.switches);
          if activated {
            mode.event_switch_closed(&switch, &mut ctx);
          } else {
            mode.event_switch_opened(&switch, &mut ctx);
          }
          machine_commands.extend(ctx.take_commands());
        }
      }

      // Game stack
      if self.game.is_started() {
        let current_player = self.game.current_player().unwrap();
        let player_store = &mut self.player_stores[current_player as usize];
        let player_game_stack = &mut self.current_game_stack[current_player as usize];
        let current_game_frame = player_game_stack.last_mut().unwrap();
        for mode in current_game_frame {
          if mode.is_listening() {
            let mut ctx = GameContext::new(
              &self.game,
              &mut self.machine_store,
              player_store,
              &self.switches,
            );
            if activated {
              mode.event_switch_closed(&switch, &mut ctx);
            } else {
              mode.event_switch_opened(&switch, &mut ctx);
            }
            machine_commands.extend(ctx.take_machine_commands());
            game_commands.extend(ctx.take_game_commands());
          }
        }
      }

      if machine_commands.len() > 0 {
        self.process_machine_commands(machine_commands);
      }
      if game_commands.len() > 0 {
        self.process_game_commands(game_commands);
      }
    } else {
      log::warn!(
        "Received event for unknown switch ID {} : {:?}",
        switch_id,
        state
      );
      return;
    }
  }

  fn process_machine_commands(&mut self, commands: Vec<MachineCommand>) {
    let old_game_state = self.game.clone();
    let mut game_state_changed = false;

    for command in commands {
      match command {
        MachineCommand::StartGame => {
          self.start_game();
          game_state_changed = true;
        }
        MachineCommand::AddPlayer => {
          self.add_player();
          game_state_changed = true;
        }
        MachineCommand::ActivateHighVoltage => {
          self.enable_watchdog();
        }
        MachineCommand::DeactivateHighVoltage => {
          self.disable_watchdog();
        }
        MachineCommand::ActivateDriver(driver) => {
          todo!();
        }
        MachineCommand::DeactivateDriver(driver) => {
          todo!();
        }
        MachineCommand::ConfigureDriver(driver, config) => {
          self.configure_driver(driver, config);
        }
        MachineCommand::TriggerDriver(driver) => {
          todo!();
        }
      }
    }

    if game_state_changed {
      let current_frame = self.machine_stack.last_mut().unwrap();
      for mode in current_frame {
        mode.on_game_state_changed(&old_game_state, &self.game, &self.switches);
      }

      if self.game.is_started() {
        let current_player = self.game.current_player().unwrap();
        let player_game_stack = &mut self.current_game_stack[current_player as usize];
        let current_game_frame = player_game_stack.last_mut().unwrap();
        for mode in current_game_frame {
          mode.on_game_state_changed(&old_game_state, &self.game, &self.switches);
        }
      }
    }
    // if game state changed, process mode events
  }

  fn process_game_commands(&mut self, commands: Vec<GameCommand>) {
    for command in commands {
      match command {
        GameCommand::AddPoints(points) => {
          let current_player = self.game.current_player().unwrap();
          self.player_points[current_player as usize] += points;
          log::debug!(
            "Added {} points to player {}. Total points: {}",
            points,
            current_player + 1,
            self.player_points[current_player as usize]
          );
        }
      }
    }
  }

  fn start_game(&mut self) {
    log::info!("Starting new game");
    self.player_points.clear();
    self.player_points.push(0);

    self.player_stores.clear();
    self.player_stores.push(Store::new());

    self.current_game_stack.clear();
    self
      .current_game_stack
      .push(self.clone_game_stack(&self.init_game_stack));

    self.game = GameState {
      started: true,
      player_count: 1,
      current_player: Some(0),
      current_ball: Some(0),
    };

    self.enable_watchdog();
  }

  fn add_player(&mut self) {
    log::info!("Adding player to game");
    self.player_stores.push(Store::new());
    self.player_points.push(0);
    self
      .current_game_stack
      .push(self.clone_game_stack(&self.init_game_stack));
    self.game.player_count += 1;
  }

  fn enable_watchdog(&mut self) {
    log::info!("Enabling watchdog");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(true));
  }

  fn disable_watchdog(&mut self) {
    log::info!("Disabling watchdog");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(false));
  }

  fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
    match self.driver_lookup.get(driver) {
      Some(driver) => {
        log::info!("Configuring driver {}", driver.name);
        let cmd = protocol::configure_driver::request(driver, config);
        let _ = self.command_tx.try_send(MainboardCommand::SendIo(cmd));
      }
      None => {
        log::error!("Attempted to configure unknown driver: {}", driver);
        return;
      }
    }
  }

  /// Deep clones a game stack by cloning each frame and each mode within the frame
  /// This ensures that each player has their own independent instances of each game mode
  fn clone_game_stack(&self, stack: &[GameFrame]) -> Vec<GameFrame> {
    stack
      .iter()
      .map(|frame| {
        frame
          .iter()
          .map(|mode| dyn_clone::clone_box(&**mode))
          .collect()
      })
      .collect()
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}
