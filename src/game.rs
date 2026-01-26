use crate::IoNetwork;
use crate::mainboard::{MainboardCommand, MainboardIncoming};
use crate::modes_old::mode::Mode;
use crate::prelude::*;
use crate::protocol::FastResponse;
use tokio::sync::mpsc;

pub mod GameState {
  pub struct Init;
  pub struct Booted;
  pub struct Attract;
  pub struct Running;
}

#[derive(Debug)]
pub struct Game<State> {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  player_count: u8,
  current_player: u8,
  _state: std::marker::PhantomData<State>,
}

#[derive(Debug, Clone)]
pub enum FastChannel {
  Io,
  Expansion,
}

impl Game<GameState::Init> {
  pub async fn boot(config: BootConfig, io_network: IoNetwork) -> Game<GameState::Booted> {
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

    // TODO: read state of all switches from mainboard and setup starting state

    Game {
      command_tx,
      event_rx,
      player_count: 1,
      current_player: 0,
    }
  }

  pub async fn run<M: Mode + 'static>(&mut self, modes: Vec<M>) {
    loop {
      match self.event_rx.recv().await {
        Some(event) => match event.data {
          FastResponse::Switch { switch_id, state } => {
            for mode in &mut modes {
              // TODO: map switch ID to Switch struct with name and other reference data
              mode.on_switch(&switch_id, state, self);
            }
          }
          _ => {
            log::warn!("Unhandled mainboard event: {:?}", event.data);
          }
        },
        None => {}
      }
    }
  }
}
