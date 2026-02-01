use std::collections::HashMap;
use std::fmt::Debug;

use crate::mainboard::{MainboardCommand, MainboardIncoming};
use crate::modes::prelude::*;
use crate::protocol::{FastResponse, SwitchState};
use crate::store::Store;
use crate::{IoNetwork, Mainboard, prelude::*};
use tokio::sync::mpsc;

pub type MachineFrame = Vec<Box<dyn MachineMode>>;

#[derive(Debug)]
pub struct Machine {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: HashMap<usize, Switch>,
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

  pub async fn run(&mut self) {
    loop {
      if let Some(event) = self.event_rx.recv().await {
        match event.data {
          FastResponse::Switch { switch_id, state } => self.run_switch_event(switch_id, state),
          _ => {
            // handle other events
          }
        }
      }
    }
  }

  fn run_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    if let Some(switch) = self.switches.get(&switch_id) {
      let activated = matches!(state, SwitchState::Closed);
      let current_frame = self.machine_stack.last_mut().unwrap();
      for mode in current_frame {
        let mut ctx = MachineContext::new(&self.game, &mut self.machine_store);
        if activated {
          mode.on_switch_activated(switch, &mut ctx);
        } else {
          mode.on_switch_deactivated(switch, &mut ctx);
        }
      }
    } else {
      log::warn!("Received event for unknown switch ID {}", switch_id);
      return;
    }
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
}
