use std::cell::RefMut;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

pub struct SystemGroup {
  pub(crate) systems: Systems,
  pub(crate) active: bool,
}

impl SystemGroup {
  pub fn new() -> Self {
    Self {
      systems: Systems::new(),
      active: true,
    }
  }

  pub fn child_ids(&self) -> Vec<&u64> {
    self.systems.ids()
  }

  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get_by_id(system_id)
  }

  pub fn activate(&mut self) {
    self.active = true;
  }

  pub fn deactivate(&mut self) {
    self.active = false;
  }
}

impl Deref for SystemGroup {
  type Target = Systems;

  fn deref(&self) -> &Self::Target {
    &self.systems
  }
}

impl DerefMut for SystemGroup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.systems
  }
}
