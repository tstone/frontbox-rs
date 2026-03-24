use std::collections::HashMap;

use crate::prelude::Storable;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Storable)]
pub struct SystemState {
  pub inner: HashMap<u64, bool>,
}

impl SystemState {
  pub fn new() -> Self {
    Self {
      inner: HashMap::new(),
    }
  }

  pub fn set_active(&mut self, system_id: u64, active: bool) {
    self.inner.insert(system_id, active);
  }

  pub fn activate(&mut self, system_id: u64) {
    self.set_active(system_id, true);
  }

  pub fn deactivate(&mut self, system_id: u64) {
    self.set_active(system_id, false);
  }

  pub fn is_active(&self, system_id: u64) -> bool {
    *self.inner.get(&system_id).unwrap_or(&false)
  }
}
