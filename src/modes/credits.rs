use crate::machine::Switch;
use crate::modes::mode::{AttractMachineRef, Mode};
use crate::protocol::SwitchState;

#[derive(Debug)]
pub struct Credits {
  config: Payment,
  credits: u8,
}

impl Credits {
  pub fn new(config: Payment) -> Self {
    Self { config, credits: 0 }
  }

  pub fn free_play() -> Self {
    Self {
      config: Payment::FreePlay,
      credits: 0,
    }
  }

  pub fn require(amount: u8) -> Self {
    Self {
      config: Payment::CreditsRequired(amount),
      credits: 0,
    }
  }
}

impl Mode<AttractMachineRef> for Credits {
  fn on_switch(&mut self, switch: &Switch, state: SwitchState, machine: &mut AttractMachineRef) {
    if state == SwitchState::Closed {
      match switch.name {
        payment_switches::COIN1
        | payment_switches::COIN2
        | payment_switches::COIN3
        | payment_switches::COIN4 => match self.config {
          Payment::CreditsRequired(required) => {
            self.credits += 1;
            log::info!("Credit added! Total credits: {}", self.credits);
            if self.credits >= required {
              log::info!("Enough credits to start a game!");
            }
          }
          _ => {}
        },
        payment_switches::START_BUTTON => match self.config {
          Payment::FreePlay => {
            machine.start_game();
          }
          Payment::CreditsRequired(required) => {
            if self.credits >= required {
              self.credits -= required;
              log::info!("Starting game! Remaining credits: {}", self.credits);
              machine.start_game();
              return;
            } else {
              log::info!("Not enough credits to start a game.");
            }
          }
        },
        _ => {}
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Payment {
  FreePlay,
  CreditsRequired(u8),
}

impl Default for Payment {
  fn default() -> Self {
    Payment::FreePlay
  }
}

pub mod payment_switches {
  pub const START_BUTTON: &str = "start_button";
  pub const COIN1: &str = "coin1";
  pub const COIN2: &str = "coin2";
  pub const COIN3: &str = "coin3";
  pub const COIN4: &str = "coin4";
}
