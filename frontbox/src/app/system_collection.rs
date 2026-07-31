use std::cell::RefMut;
use std::collections::HashMap;

use crate::prelude::*;

/// Holds all the systems used by the framework's run loop
pub struct SystemCollection {
  pub systems: Systems,
  pub groups: HashMap<&'static str, SystemGroup>,
}

impl SystemCollection {
  pub fn get_by_id(&'_ mut self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    // check systems first
    if self.systems.contains_id(system_id) {
      return self.systems.get_by_id(system_id);
    } else {
      // if not search groups
      for (_, group) in &self.groups {
        if group.contains_id(system_id) {
          return group.systems.get_by_id(system_id);
        }
      }
    }
    None
  }

  pub fn parent(&'_ mut self, system_id: &u64) -> Option<&mut Systems> {
    if self.systems.contains_id(system_id) {
      Some(&mut self.systems)
    } else if let Some(group) = self.parent_group(&system_id) {
      Some(group)
    } else {
      None
    }
  }

  /// Finds the parent system group that a system is in, if any
  pub fn parent_group(&'_ mut self, system_id: &u64) -> Option<&mut SystemGroup> {
    self.groups.values_mut()
      .find(|g| g.contains_id(system_id))
  }
}
