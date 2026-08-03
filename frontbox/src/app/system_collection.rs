use std::cell::RefMut;
use std::collections::HashMap;

use crate::prelude::*;

/// Holds all the systems used by the framework's run loop
pub struct SystemCollection {
  pub systems: Systems,
  pub groups: HashMap<&'static str, SystemGroup>,
}

impl SystemCollection {
  pub fn get_by_id(&self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
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

  pub fn get_by_type<T: System + 'static>(&self) -> Option<RefMut<'_, T>> {
    if let Some(system) = self.systems.get_by_type::<T>() {
      return Some(system);
    } else {
      // if not search groups
      for (_, group) in &self.groups {
        if let Some(system) = group.systems.get_by_type::<T>() {
          return Some(system);
        }
      }
    }

    None
  }

  pub fn parent_of(&'_ self, system_id: &u64) -> Option<&Systems> {
    log::debug!(
      "Checking if system is in root: {}, {:?}",
      system_id,
      self.systems.ids()
    );
    if self.systems.contains_id(system_id) {
      Some(&self.systems)
    } else if let Some(group) = self.parent_group(system_id) {
      Some(group)
    } else {
      log::warn!("No parent found for {}", system_id);
      None
    }
  }

  pub fn parent_of_mut(&'_ mut self, system_id: &u64) -> Option<&mut Systems> {
    if self.systems.contains_id(system_id) {
      Some(&mut self.systems)
    } else if let Some(group) = self.parent_group_mut(&system_id) {
      Some(group)
    } else {
      log::warn!("No parent found for {}", system_id);
      None
    }
  }

  /// Finds the parent system group that a system is in, if any
  pub fn parent_group(&'_ self, system_id: &u64) -> Option<&SystemGroup> {
    self.groups.values().find(|g| {
      log::debug!(
        "Checking if group contains {}, {:?}",
        system_id,
        g.child_ids(),
      );
      g.contains_id(system_id)
    })
  }

  /// Finds the parent system group that a system is in, if any
  pub fn parent_group_mut(&'_ mut self, system_id: &u64) -> Option<&mut SystemGroup> {
    self.groups.values_mut().find(|g| g.contains_id(system_id))
  }
}
