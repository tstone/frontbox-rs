use std::collections::HashMap;
use std::fmt::Debug;

use crate::modes::mode::{AttractMachineRef, AttractNextState, Mode};
use crate::mainboard::{MainboardCommand, MainboardIncoming};
use crate::prelude::*;
use crate::protocol::FastResponse;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Machine {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: HashMap<usize, Switch>,
  state: MachineState,
  credits: u8,
  attract_modes: Vec<Box<dyn Mode + Send>>,
}

#[derive(Debug, Clone)]
pub enum FastChannel {
  Io,
  Expansion,
}

impl Machine {
  pub async fn boot(config: BootConfig) -> Self {
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
    // TODO: send switch configuration (debounce, invert, etc.)
    // TODO: read state of all switches from mainboard and setup starting state

    Self {
      command_tx,
      event_rx,
      switches: HashMap::new(),
      state: MachineState::Attract,
      credits: 0,
      attract_modes: Vec::new(),
    }
  }

  pub fn with_attract_mode<T: Mode + Send + 'static>(&mut self, mode: T) {
    self.attract_modes.push(Box::new(mode));
  }

  pub fn with_attract_modes<T: Mode + Send + 'static>(&mut self, modes: Vec<T>) {
    for mode in modes {
      self.attract_modes.push(Box::new(mode));
    }
  }

  pub async fn run(&mut self) {
    loop {
      match self.state {
        MachineState::Attract => self.receive_attract().await,
        MachineState::InGame => self.receive_game().await,
        MachineState::Config => { /* TODO */ }
      }
    }
  }

  async fn receive_attract(&mut self) {
    if let Some(event) = self.event_rx.recv().await {
      match event.data {
        FastResponse::Switch { switch_id, state } => {
          if let Some(switch) = self.switches.get(&switch_id) {
            for mode in self.attract_modes.iter_mut() {
              let next_state = mode.on_switch(switch, &state, &mut AttractMachineRef {});
              match next_state {
                AttractNextState::EnterGame => {
                  self.state = MachineState::InGame;
                  log::info!("Entering game state from attract mode");
                  break;
                }
                AttractNextState::EnterConfig => {
                  self.state = MachineState::Config;
                  log::info!("Entering config state from attract mode");
                  break;
                }
                AttractNextState::None => { /* stay in attract mode */ }
              }
            }
          }
        }
        _ => { /* handle other events if necessary */ }
      }
    }
  }

  async fn receive_game(&mut self) {
    if let Some(event) = self.event_rx.recv().await {
      // TODO
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MachineState {
  Attract,
  Config,
  InGame,
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}
