use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::SwitchDefinition;
use crate::prelude::*;
use fast_protocol::SwitchState;

#[derive(Clone)]
pub struct SwitchLookup {
  by_id: HashMap<usize, Switch>,
  pub(crate) by_name: HashMap<&'static str, Switch>,
  is_closed: HashMap<usize, bool>,
  configs: HashMap<usize, SwitchConfig>,
}

impl SwitchLookup {
  pub fn new(definitions: Vec<SwitchDefinition>, initial_state: Vec<SwitchState>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut is_closed = HashMap::new();
    let mut configs = HashMap::new();
    for definition in definitions {
      by_id.insert(
        definition.id,
        Switch {
          id: definition.id,
          name: definition.name,
          native: definition.native.clone(),
          tags: definition.tags.clone(),
        },
      );

      by_name.insert(
        definition.name,
        Switch {
          id: definition.id,
          name: definition.name,
          native: definition.native.clone(),
          tags: definition.tags.clone(),
        },
      );

      if let Some(config) = definition.config {
        configs.insert(definition.id, config);
      }

      // Actual state is populated below from initial report
      is_closed.insert(definition.id, false);
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

  pub fn is_open_by_id(&self, switch_id: usize) -> Option<bool> {
    self.is_closed.get(&switch_id).map(|closed| !closed)
  }

  pub fn is_closed_by_id(&self, switch_id: usize) -> Option<bool> {
    self.is_closed.get(&switch_id).copied()
  }

  pub fn is_closed(&self, switch_name: &'static str) -> Option<bool> {
    self
      .by_name
      .get(switch_name)
      .and_then(|switch| self.is_closed_by_id(switch.id))
  }

  pub fn is_open(&self, switch_name: &'static str) -> Option<bool> {
    self
      .by_name
      .get(switch_name)
      .and_then(|switch| self.is_open_by_id(switch.id))
  }

  pub fn by_id(&self, switch_id: &usize) -> Option<&Switch> {
    self.by_id.get(switch_id)
  }

  pub fn by_id_mut(&mut self, switch_id: &usize) -> Option<&mut Switch> {
    self.by_id.get_mut(switch_id)
  }

  pub fn by_name(&self, switch_name: &'static str) -> Option<&Switch> {
    self.by_name.get(switch_name)
  }

  pub fn by_name_mut(&mut self, switch_name: &'static str) -> Option<&mut Switch> {
    self.by_name.get_mut(switch_name)
  }

  pub fn by_tag<T: HardwareTag + 'static>(&self) -> Vec<&Switch> {
    self
      .by_id
      .values()
      .filter(|switch| {
        switch
          .tags
          .iter()
          .any(|tag| <dyn HardwareTag>::as_any(tag.as_ref()).is::<T>())
      })
      .collect()
  }

  pub fn by_selection(&self, selection: &HardwareSelection) -> Vec<&Switch> {
    self
      .by_id
      .values()
      .filter(|switch| selection.matches_switch(switch))
      .collect()
  }

  /// Used internally to update switch state via switch events
  pub(crate) fn update_switch_state(&mut self, switch_id: usize, state: SwitchState) {
    let is_closed = matches!(state, SwitchState::Closed);
    self.is_closed.insert(switch_id, is_closed);
  }

  pub fn config(&self, name: &'static str) -> Option<&SwitchConfig> {
    self
      .by_name
      .get(name)
      .and_then(|switch| self.configs.get(&switch.id))
  }

  #[allow(unused)]
  pub(crate) fn update_switch_config(&mut self, switch_id: usize, config: SwitchConfig) {
    self.configs.insert(switch_id, config);
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

impl SwitchNameToId for SwitchLookup {
  fn switch_id(&self, name: &str) -> Option<usize> {
    self.by_name.get(name).map(|switch| switch.id)
  }
}

impl Deref for SwitchLookup {
  type Target = HashMap<&'static str, Switch>;
  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for SwitchLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}

#[derive(Debug, Clone)]
pub struct Switch {
  pub id: usize,
  pub native: NativeIdentity,
  pub name: &'static str,
  pub tags: Vec<Box<dyn HardwareTag>>,
}

impl Switch {
  pub fn has_tag<T: HardwareTag + 'static>(&self) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn HardwareTag>::as_any(tag.as_ref()).is::<T>())
  }

  pub(crate) fn has_typed_tag(&self, type_id: TypeId) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn HardwareTag>::as_any(tag.as_ref()).type_id() == type_id)
  }
}

#[cfg(test)]
mod tests {
  use crate::NativeIdentity;
  use crate::tags::{FlipperButton, Playfield};

  use super::*;

  #[test]
  fn tag_lookup() {
    let lookup = SwitchLookup::new(
      vec![SwitchDefinition {
        id: 1,
        name: "switch1",
        native: NativeIdentity::new(0, 1),
        tags: vec![Box::new(Playfield)],
        config: None,
      }],
      vec![SwitchState::Open],
    );

    let switches = lookup.by_tag::<Playfield>();
    assert_eq!(switches.len(), 1);
    assert_eq!(switches[0].name, "switch1");
  }

  #[test]
  fn switch_has_tag() {
    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    assert!(switch.has_tag::<Playfield>());
    assert!(!switch.has_tag::<FlipperButton>());
  }

  #[test]
  fn switch_has_tag_type_id() {
    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    let type_id = TypeId::of::<Playfield>();
    assert!(switch.has_typed_tag(type_id));
  }
}
