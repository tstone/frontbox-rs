use std::collections::HashMap;
use std::fmt::Debug;

use crate::mainboard::{MainboardCommand, MainboardIncoming};
use crate::modes::machine_context::MachineCommand;
use crate::modes::prelude::*;
use crate::protocol::{FastResponse, SwitchState};
use crate::store::Store;
use crate::{IoNetwork, Mainboard, prelude::*};
use crossterm::{
  event::{Event, EventStream, KeyCode},
  terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub type MachineFrame = Vec<Box<dyn MachineMode>>;

#[derive(Debug)]
pub struct Machine {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: HashMap<usize, Switch>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  machine_stack: Vec<MachineFrame>,
  machine_store: Store,
  player_stores: Vec<Store>,
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

    let mut switches = HashMap::new();
    for switch in io_network.switches {
      // TODO: send switch configuration (debounce, invert, etc.)
      switches.insert(
        switch.id,
        Switch {
          id: switch.id,
          name: switch.name,
        },
      );
    }

    Self {
      command_tx,
      event_rx,
      switches,
      keyboard_switch_map: HashMap::new(),
      machine_stack: Vec::new(),
      player_stores: Vec::new(),
      machine_store: Store::new(),
      game: GameState::default(),
    }
  }

  pub fn add_machine_frame(&mut self, frame: MachineFrame) -> &mut Self {
    self.machine_stack.push(frame);
    self
  }

  pub fn add_keyboard_mapping(&mut self, key: KeyCode, switch_name: &'static str) -> &mut Self {
    let id = self
      .switches
      .values()
      .find(|s| s.name == switch_name)
      .map(|s| s.id)
      .unwrap();
    self.keyboard_switch_map.insert(key, id);
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
    if let Some(switch) = self.switches.get(&switch_id) {
      let mut commands = Vec::new();

      let activated = matches!(state, SwitchState::Closed);
      let current_frame = self.machine_stack.last_mut().unwrap();
      for mode in current_frame {
        if mode.is_active() {
          let mut ctx = MachineContext::new(&self.game, &mut self.machine_store);
          if activated {
            mode.on_switch_activated(switch, &mut ctx);
          } else {
            mode.on_switch_deactivated(switch, &mut ctx);
          }
          commands.extend(ctx.take_commands());
        }
      }

      if commands.len() > 0 {
        self.process_commands(commands);
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

  fn process_commands(&mut self, commands: Vec<MachineCommand>) {
    let old_game_state = self.game.clone();
    let mut game_state_changed = false;

    for command in commands {
      match command {
        MachineCommand::StartGame => {
          log::info!("Starting new game");
          self.player_stores.clear();
          self.player_stores.push(Store::new());
          self.game = GameState {
            started: true,
            player_count: 1,
            current_player: Some(0),
            current_ball: Some(0),
          };
          self.enable_watchdog();
          game_state_changed = true;
        }
        MachineCommand::AddPlayer => {
          log::info!("Adding player to game");
          self.player_stores.push(Store::new());
          self.game.player_count += 1;
          game_state_changed = true;
        }
        MachineCommand::ActivateHighVoltage => {
          self.enable_watchdog();
        }
        MachineCommand::DeactivateHighVoltage => {
          self.disable_watchdog();
        }
      }
    }

    if game_state_changed {
      let current_frame = self.machine_stack.last_mut().unwrap();
      for mode in current_frame {
        mode.on_game_state_changed(&old_game_state, &self.game);
      }
    }
    // if game state changed, process mode events
  }

  fn enable_watchdog(&mut self) {
    log::info!("Enabling watchdog");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(true));
  }

  fn disable_watchdog(&mut self) {
    log::info!("Disabling watchdog");
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(false));
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}
