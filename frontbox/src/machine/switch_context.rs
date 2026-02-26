use std::collections::HashMap;

use crate::SwitchSpec;
use crate::prelude::*;
use fast_protocol::SwitchState;

#[derive(Debug)]
pub struct SwitchContext {
  by_id: HashMap<usize, Switch>,
  by_name: HashMap<&'static str, Switch>,
  is_closed: HashMap<usize, bool>,
  configs: HashMap<usize, SwitchConfig>,
}

impl SwitchContext {
  pub fn new(switch_specs: Vec<SwitchSpec>, initial_state: Vec<SwitchState>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut is_closed = HashMap::new();
    let mut configs = HashMap::new();

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

      if let Some(config) = spec.config {
        configs.insert(spec.id, config);
      }

      // Actual state is populated below from initial report
      is_closed.insert(spec.id, false);
    }

    let mut context = Self {
      by_id,
      by_name,
      is_closed,
      configs,
    };

    // set initial states
    context.update_switch_states(initial_state);
    context
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

  /// Used internally to define an additional virtual (non-hardware backed) switch
  pub(crate) fn add_virtual_switch(&mut self, switch_name: &'static str, id: usize) {
    let switch = Switch {
      id,
      name: switch_name,
    };
    self.by_id.insert(id, switch.clone());
    self.by_name.insert(switch_name, switch);
    self.is_closed.insert(id, false);
  }

  /// Used internally to update switch state via switch events
  pub(crate) fn update_switch_state(&mut self, switch_id: usize, state: SwitchState) {
    let is_closed = matches!(state, SwitchState::Closed);
    self.is_closed.insert(switch_id, is_closed);
  }

  /// Used internally to update all switch states based on a switch report
  pub(crate) fn update_switch_states(&mut self, states: Vec<SwitchState>) {
    for (switch_id, state) in states.into_iter().enumerate() {
      // Switch report does not account for switch config inversion
      // https://fastpinball.com/fast-serial-protocol/net/sa/
      let mut invert = false;
      if let Some(config) = self.configs.get(&switch_id) {
        invert = config.inverted;
      }

      let adjusted_state = if invert {
        match state {
          SwitchState::Open => SwitchState::Closed,
          SwitchState::Closed => SwitchState::Open,
        }
      } else {
        state
      };

      self.update_switch_state(switch_id, adjusted_state);
    }
  }
}
