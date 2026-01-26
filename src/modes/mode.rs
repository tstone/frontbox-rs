use std::fmt::Debug;

use crate::machine::Switch;
use crate::protocol::SwitchState;

#[derive(Debug)]
pub struct AttractMachineRef;

impl AttractMachineRef {
  pub fn start_game(&mut self) {
    todo!();
  }
}

pub trait Mode<M>: Debug {
  fn on_switch(&mut self, switch: &Switch, state: SwitchState, machine: &mut M);
}
