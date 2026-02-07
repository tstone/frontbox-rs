use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::hardware::driver_config::DriverConfig;
use crate::mainboard::*;
use crate::prelude::*;
use crate::protocol::*;
use crate::runtimes::*;
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
  pub(crate) runtime_stack: Vec<Box<dyn Runtime>>,
  pub(crate) active_player: i8,
  pub(crate) active_player_count: i8,
}

impl Machine {
  pub fn runtime_type(&self) -> RuntimeType {
    Any::type_id(&*self.runtime_stack.last().unwrap())
  }

  pub fn active_store(&mut self) -> &mut Store {
    let runtime = self.runtime_stack.last_mut().unwrap();
    let (_scene, store) = runtime.get_current();
    store
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }

  pub fn is_game_started(&self) -> bool {
    self.active_player >= 0
  }

  pub fn active_player(&self) -> Option<u8> {
    if self.active_player >= 0 {
      Some(self.active_player as u8)
    } else {
      None
    }
  }

  // ---

  pub async fn run(&mut self, runtime: Box<dyn Runtime>) {
    self.push_runtime(runtime);

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

      self.dispatch_to_modes(|mode, ctx| {
        if activated {
          mode.on_switch_closed(&switch, ctx);
        } else {
          mode.on_switch_opened(&switch, ctx);
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
    self.dispatch_to_modes(|mode, ctx| {
      mode.on_game_start(ctx);
    });
  }

  fn run_on_game_end(&mut self) {
    self.dispatch_to_modes(|mode, ctx| {
      mode.on_game_end(ctx);
    });
  }

  fn run_on_ball_start(&mut self) {
    self.dispatch_to_modes(|mode, ctx| {
      mode.on_ball_start(ctx);
    });
  }

  fn run_on_ball_end(&mut self) {
    self.dispatch_to_modes(|mode, ctx| {
      mode.on_ball_end(ctx);
    });
  }

  /// Run each system within the scene, capturing then running commands emitted during processing
  fn dispatch_to_modes<F>(&mut self, mut handler: F)
  where
    F: FnMut(&mut Box<dyn System>, &mut Context),
  {
    let runtime_type = self.runtime_type();
    let current_player = self.active_player();
    let runtime = self.runtime_stack.last_mut().unwrap();
    let (scene, store) = runtime.get_current();
    let mut ctx = Context::new(runtime_type, current_player, store, &self.switches);

    for system in scene.iter_mut() {
      handler(system, &mut ctx);
    }

    let mut commands = Vec::new();
    commands.extend(ctx.take_commands());
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

  pub(crate) fn start_game(&mut self) {
    log::info!("Starting new game");
    self.active_player = 0;
    self.run_on_game_start();
    self.enable_high_voltage();
  }

  pub fn add_player(&mut self) {
    log::info!("Adding player to game");
    self.active_player_count += 1;
  }

  pub fn advance_player(&mut self) {
    log::info!("Advancing to next player");

    if self.is_game_started() {
      self.run_on_ball_end();
      self.active_player += 1;
      if self.active_player >= self.active_player_count {
        self.active_player = 0;
      }
      self.run_on_ball_start();
    }
  }

  /// Transition to a new runtime
  pub fn push_runtime(&mut self, new_runtime: Box<dyn Runtime>) {
    log::info!("Transitioning to new machine runtime");
    let runtime = self.runtime_stack.last_mut();
    if let Some(runtime) = runtime {
      let mut ctx = RuntimeContext::new();
      runtime.on_runtime_exit(&mut ctx);
      self.execute_runtime_commands(ctx.commands());
    }

    let mut ctx = RuntimeContext::new();
    new_runtime.on_runtime_enter(&mut ctx);
    self.execute_runtime_commands(ctx.commands());

    self.runtime_stack.push(new_runtime);
  }

  pub fn pop_runtime(&mut self) {
    if self.runtime_stack.len() <= 1 {
      log::warn!("Attempted to pop runtime, but only one runtime exists");
      return;
    }

    log::info!("Popping current machine runtime");
    let mut ctx = RuntimeContext::new();
    let runtime = self.runtime_stack.last_mut().unwrap();
    runtime.on_runtime_exit(&mut ctx);
    self.execute_runtime_commands(ctx.commands());

    self.runtime_stack.pop();

    let mut ctx = RuntimeContext::new();
    let runtime = self.runtime_stack.last_mut().unwrap();
    runtime.on_runtime_enter(&mut ctx);
    self.execute_runtime_commands(ctx.commands());
  }

  fn execute_runtime_commands(&mut self, commands: Vec<RuntimeCommand>) {
    for command in commands {
      match command {
        RuntimeCommand::StartGame => self.start_game(),
      }
    }
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
