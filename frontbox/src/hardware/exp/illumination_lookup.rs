use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::hardware::exp::addressable_illumination::*;
use crate::{HardwareQuery, HardwareTag, ResolvedExpansionBoard};

#[derive(Debug, Clone)]
pub struct IlluminationLookup {
  by_name: HashMap<&'static str, AddressableIllumination>,
}

impl IlluminationLookup {
  pub fn new(boards: &Vec<ResolvedExpansionBoard>) -> Self {
    let mut by_name = HashMap::new();

    for board in boards {
      for port in &board.led_ports {
        for illum in &port.illuminations {
          by_name.insert(illum.name(), illum.clone());
        }
      }
    }

    Self { by_name }
  }

  pub fn by_name(&self, name: &str) -> Option<&AddressableIllumination> {
    self.by_name.get(name)
  }

  pub fn by_tag<T: HardwareTag + 'static>(&self) -> Vec<&AddressableIllumination> {
    self
      .by_name
      .values()
      .filter(|illum| {
        illum
          .tags()
          .iter()
          .any(|tag| <dyn HardwareTag>::as_any(&**tag).is::<T>())
      })
      .collect()
  }

  pub fn query(&self, selection: &HardwareQuery) -> Vec<&AddressableIllumination> {
    self
      .by_name
      .values()
      .filter(|illum| selection.matches_illumination(illum))
      .collect()
  }
}

impl Deref for IlluminationLookup {
  type Target = HashMap<&'static str, AddressableIllumination>;

  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for IlluminationLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}
