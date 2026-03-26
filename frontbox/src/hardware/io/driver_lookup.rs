use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Clone)]
pub struct DriverLookup {
  by_id: HashMap<usize, Driver>,
  by_name: HashMap<&'static str, Driver>,
  configs: HashMap<usize, Box<dyn DriverMode>>,
  tags: HashMap<usize, Vec<Box<dyn HardwareTag>>>,
}

impl DriverLookup {
  pub fn new(drivers: Vec<DriverDefinition>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();
    let mut tags = HashMap::new();

    for definition in drivers {
      by_id.insert(
        definition.id,
        Driver {
          id: definition.id,
          name: definition.name,
          native: definition.native.clone(),
        },
      );

      by_name.insert(
        definition.name,
        Driver {
          id: definition.id,
          name: definition.name,
          native: definition.native.clone(),
        },
      );

      if let Some(config) = definition.mode {
        configs.insert(definition.id, config);
      }
      tags.insert(definition.id, definition.tags);
    }

    Self {
      by_id,
      by_name,
      configs,
      tags,
    }
  }

  pub fn driver_by_id(&self, driver_id: &usize) -> Option<&Driver> {
    self.by_id.get(driver_id)
  }

  pub fn driver_by_id_mut(&mut self, driver_id: &usize) -> Option<&mut Driver> {
    self.by_id.get_mut(driver_id)
  }

  pub fn driver_by_name(&self, driver_name: &'static str) -> Option<&Driver> {
    self.by_name.get(driver_name)
  }

  pub fn driver_by_name_mut(&mut self, driver_name: &'static str) -> Option<&mut Driver> {
    self.by_name.get_mut(driver_name)
  }

  pub fn driver_config(&self, name: &'static str) -> Option<&Box<dyn DriverMode>> {
    self
      .by_name
      .get(name)
      .and_then(|driver| self.configs.get(&driver.id))
  }

  pub fn drivers_by_tag<T: HardwareTag + 'static>(&self) -> Vec<&Driver> {
    self
      .by_id
      .values()
      .filter(|driver| {
        self
          .tags
          .get(&driver.id)
          .map(|tags| {
            tags
              .iter()
              .any(|tag| <dyn HardwareTag>::as_any(&**tag).is::<T>())
          })
          .unwrap_or(false)
      })
      .collect()
  }
}

impl Deref for DriverLookup {
  type Target = HashMap<&'static str, Driver>;
  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for DriverLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}

#[derive(Clone)]
pub struct Driver {
  pub id: usize,
  pub name: &'static str,
  pub native: NativeIdentity,
}
