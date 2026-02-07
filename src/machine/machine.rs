use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::hardware::driver_config::DriverConfig;
use crate::mainboard::*;
use crate::prelude::*;
use crate::protocol::*;
use crate::runtimes::*;
use crate::store::Store;
use crossterm::{
  event::{Event, EventStream, KeyCode},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub type Scene = Vec<Box<dyn System>>;

pub struct Machine {
  pub(crate) command_tx: mpsc::Sender<MainboardCommand>,
  pub(crate) event_rx: mpsc::Receiver<MainboardIncoming>,
  pub(crate) switches: SwitchContext,
  pub(crate) driver_lookup: HashMap<&'static str, DriverPin>,
  pub(crate) keyboard_switch_map: HashMap<KeyCode, usize>,
  pub(crate) store: Store,
  pub(crate) game_state: Option<GameState>,
  pub(crate) runtime: Box<dyn Runtime>,
}

impl Machine {
  pub fn runtime_type(&self) -> MachineModeType {
    Any::type_id(&*self.runtime)
  }

  pub fn game(&mut self) -> Option<&mut GameState> {
    self.game_state.as_mut()
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }

  pub fn get<T: Default + 'static>(&mut self) -> &T {
    self.store.get_state::<T>()
  }

  pub fn get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.store.get_state_mut::<T>()
  }

  pub fn insert<T: Default + 'static>(&mut self, value: T) {
    self.store.insert_state::<T>(value);
  }

  pub fn remove<T: Default + 'static>(&mut self) {
    self.store.remove_state::<T>();
  }

  // ---

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
            FastResponse::SwitchReport { switches } => {
              self.switches.update_switch_states(switches);
            }
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

  // ---

  fn run_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    if let Some(switch) = self.switches.switch_by_id(&switch_id).cloned() {
      self.switches.update_switch_state(switch_id, state);
      let activated = matches!(state, SwitchState::Closed);

      self.dispatch_to_modes(|mode, ctx, game| {
        if activated {
          mode.event_switch_closed(&switch, ctx, game);
        } else {
          mode.event_switch_opened(&switch, ctx, game);
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

  fn run_on_game_start(&mut self) {
    self.dispatch_to_modes(|mode, ctx, game| {
      if let Some(game) = game {
        mode.on_game_start(ctx, game);
      }
    });
  }

  fn run_on_game_end(&mut self) {
    self.dispatch_to_modes(|mode, ctx, game| {
      if let Some(game) = game {
        mode.on_game_end(ctx, game);
      }
    });
  }

  fn run_on_ball_start(&mut self) {
    self.dispatch_to_modes(|mode, ctx, game| {
      if let Some(game) = game {
        mode.on_ball_start(ctx, game);
      }
    });
  }

  fn run_on_ball_end(&mut self) {
    self.dispatch_to_modes(|mode, ctx, game| {
      if let Some(game) = game {
        mode.on_ball_end(ctx, game);
      }
    });
  }

  /// Run each system within the scene, capturing then running commands emitted during processing
  fn dispatch_to_modes<F>(&mut self, mut handler: F)
  where
    F: FnMut(&mut Box<dyn System>, &mut Context, Option<&mut GameState>),
  {
    let mut commands = Vec::new();
    let mode_type = self.runtime_type();

    for system in self.runtime.current_scene() {
      if system.is_listening() {
        let mut ctx = Context::new(mode_type, &mut self.store, &self.switches);
        handler(system, &mut ctx, self.game_state.as_mut());
        commands.extend(ctx.take_commands());
      }
    }

    if !commands.is_empty() {
      self.process_commands(commands);
    }
  }

  fn process_commands(&mut self, commands: Vec<Box<dyn Command + 'static>>) {
    // TODO: handle uniqueness

    for command in commands {
      command.execute(self);
    }
  }

  // fn process_machine_commands(&mut self, commands: Vec<MachineCommand>) {
  //   let old_game_state = self.game_state.clone();
  //   let mut game_state_changed = false;

  //   for command in commands {
  //     match command {
  //       MachineCommand::StartGame => {
  //         self.start_game();
  //         game_state_changed = true;
  //       }
  //       MachineCommand::AddPlayer => {
  //         self.add_player();
  //         game_state_changed = true;
  //       }
  //       MachineCommand::ActivateHighVoltage => {
  //         self.enable_high_voltage();
  //       }
  //       MachineCommand::DeactivateHighVoltage => {
  //         self.disable_watchdog();
  //       }
  //       MachineCommand::ActivateDriver(driver) => {
  //         todo!();
  //       }
  //       MachineCommand::DeactivateDriver(driver) => {
  //         todo!();
  //       }
  //       MachineCommand::ConfigureDriver(driver, config) => {
  //         self.configure_driver(driver, config);
  //       }
  //       MachineCommand::TriggerDriver(driver) => {
  //         todo!();
  //       }
  //       MachineCommand::AddPoints(points) => {
  //         if let Some(current_player) = self.game_state.current_player() {
  //           self.player_points[current_player as usize] += points;
  //           log::debug!(
  //             "Added {} points to player {}. Total points: {}",
  //             points,
  //             current_player + 1,
  //             self.player_points[current_player as usize]
  //           );
  //         }
  //       }
  //       MachineCommand::NextPlayer => {
  //         if self.game_state.is_started() {
  //           self.report_switches(); // re-sync switch states before changing player

  //           log::debug!("Moving to next player");
  //           let mut next_player = self.game_state.current_player().unwrap() + 1;
  //           if next_player >= self.game_state.player_count {
  //             next_player = 0;
  //           }
  //           self.game_state.current_player = Some(next_player);
  //           game_state_changed = true;
  //         }
  //       }
  //     }
  //   }

  pub fn start_game(&mut self, team_count: u8) {
    log::info!("Starting new game");

    let mut player_team_map: HashMap<u8, u8> = HashMap::new();
    player_team_map.insert(0, 0);

    self.game_state = Some(GameState::new(1, team_count, player_team_map));
    self.enable_high_voltage();
  }

  pub fn add_player(&mut self, team: Option<u8>) {
    log::info!("Adding player to game");
    if let Some(game_state) = &mut self.game_state {
      game_state.add_player(team);
      self.runtime.on_add_player(game_state.player_count - 1);
    }
  }

  pub fn transition_to_runtime(&mut self, runtime: Box<dyn Runtime>) {
    log::info!("Transitioning to new machine runtime");
    let mut old_runtime = std::mem::replace(&mut self.runtime, runtime);
    old_runtime.on_runtime_exit(self);
  }

  pub fn enable_high_voltage(&mut self) {
    log::info!("Enabling high voltage");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(true));
  }

  pub fn disable_high_voltage(&mut self) {
    log::info!("Disabling high voltage");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(false));
  }

  pub fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
    match self.driver_lookup.get(driver) {
      Some(driver) => {
        log::info!("Configuring driver {}", driver.name);
        let cmd = configure_driver::request(driver, config);
        let _ = self.command_tx.try_send(MainboardCommand::SendIo(cmd));
      }
      None => {
        log::error!("Attempted to configure unknown driver: {}", driver);
        return;
      }
    }
  }

  fn report_switches(&mut self) {
    let cmd = report_switches::request();
    match self.command_tx.try_send(MainboardCommand::SendIo(cmd)) {
      Ok(_) => {}
      Err(e) => {
        log::error!("Failed to send report switches command: {}", e);
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}
