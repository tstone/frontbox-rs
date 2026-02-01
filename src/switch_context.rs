use std::collections::HashMap;

use crate::protocol::SwitchState;
use crate::{Switch, SwitchSpec};

#[derive(Debug)]
pub struct SwitchContext {
  by_id: HashMap<usize, Switch>,
  by_name: HashMap<&'static str, Switch>,
  is_closed: HashMap<usize, bool>,
}

impl SwitchContext {
  pub fn new(switch_specs: Vec<SwitchSpec>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut is_closed = HashMap::new();

    for spec in switch_specs {
      by_id.insert(
        spec.id,
        Switch {
          id: spec.id,
          name: spec.name,
        },
      );

      by_name.insert(
        spec.name,
        Switch {
          id: spec.id,
          name: spec.name,
        },
      );

      is_closed.insert(spec.id, false); // TODO: read actual state
    }

    Self {
      by_id,
      by_name,
      is_closed,
    }
  }

  pub fn is_open(&self, switch_id: usize) -> Option<bool> {
    self.is_closed.get(&switch_id).map(|closed| !closed)
  }

  pub fn is_closed(&self, switch_id: usize) -> Option<bool> {
    self.is_closed.get(&switch_id).copied()
  }

  pub fn is_closed_by_name(&self, switch_name: &'static str) -> Option<bool> {
    self
      .by_name
      .get(switch_name)
      .and_then(|switch| self.is_closed(switch.id))
  }

  pub fn is_open_by_name(&self, switch_name: &'static str) -> Option<bool> {
    self
      .by_name
      .get(switch_name)
      .and_then(|switch| self.is_open(switch.id))
  }

  pub fn switch_by_id(&self, switch_id: &usize) -> Option<&Switch> {
    self.by_id.get(&switch_id)
  }

  pub fn switch_by_name(&self, switch_name: &'static str) -> Option<&Switch> {
    self.by_name.get(switch_name)
  }

  pub(crate) fn update_switch_state(&mut self, switch_id: usize, state: SwitchState) {
    let is_closed = matches!(state, SwitchState::Closed);
    self.is_closed.insert(switch_id, is_closed);
  }

  pub(crate) fn update_switch_states(&mut self, states: Vec<SwitchState>) {
    for (index, state) in states.into_iter().enumerate() {
      let switch_id = index + 1; // Switch IDs are 1-based
      // https://fastpinball.com/fast-serial-protocol/net/sa/
      // TODO: this does not account for switch config inversion !!!
      self.update_switch_state(switch_id, state);
    }
  }
}

pub struct StatefulSwitch {
  pub id: usize,
  pub name: &'static str,
  pub state: SwitchState,
}
