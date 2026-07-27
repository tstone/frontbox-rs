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
}
