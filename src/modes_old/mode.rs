use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::SwitchSpec;
use crate::protocol::SwitchState;

pub struct GameLink {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_tx: broadcast::Sender<MainboardIncoming>,
}

pub trait Mode {
  fn on_switch(&mut self, switch: &usize, state: SwitchState, game: &GameRef);
  fn get_state(&self, game: &GameRef) -> ModeState;
}

pub enum GameEvent {
  SwitchChanged {
    switch: SwitchSpec,
    state: SwitchState,
  },
}

pub enum ModeState {
  Active,
  ActiveExclusive,
  Complete,
}

pub trait ModeTimer: Mode {
  fn on_complete(&mut self, timer: &'static str);
}
