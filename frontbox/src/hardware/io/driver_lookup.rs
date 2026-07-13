use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Clone)]
pub struct DriverLookup {
  by_id: HashMap<usize, Driver>,
  by_name: HashMap<&'static str, Driver>,
  configs: HashMap<usize, Box<dyn DriverMode>>,
}

impl DriverLookup {
  pub fn new(drivers: Vec<IoAddressed<DriverDefinition>>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();

    for addressed in drivers {
      let driver = Driver {
        id: addressed.id,
        name: addressed.definition.name,
        assignment: addressed.assignment.clone(),
        tags: addressed.definition.tags.clone(),
        location: addressed.definition.location(),
      };

      by_id.insert(addressed.id, driver.clone());
      by_name.insert(addressed.definition.name, driver);

      if let Some(config) = addressed.definition.mode {
        configs.insert(addressed.id, config);
      }
    }

    Self {
      by_id,
      by_name,
      configs,
    }
  }

  pub fn by_id(&self, driver_id: &usize) -> Option<&Driver> {
    self.by_id.get(driver_id)
  }

  pub fn by_id_mut(&mut self, driver_id: &usize) -> Option<&mut Driver> {
    self.by_id.get_mut(driver_id)
  }

  pub fn by_name(&self, driver_name: &'static str) -> Option<&Driver> {
    self.by_name.get(driver_name)
  }

  pub fn by_name_mut(&mut self, driver_name: &'static str) -> Option<&mut Driver> {
    self.by_name.get_mut(driver_name)
  }

  pub fn config(&self, name: &'static str) -> Option<&Box<dyn DriverMode>> {
    self
      .by_name
      .get(name)
      .and_then(|driver| self.configs.get(&driver.id))
  }

  pub fn by_tag<T: Tag + 'static>(&self) -> Vec<&Driver> {
    self
      .by_id
      .values()
      .filter(|driver| {
        driver
          .tags
          .iter()
          .any(|tag| <dyn Tag>::as_any(&**tag).is::<T>())
      })
      .collect()
  }

  pub fn by_selection(&self, selection: &HardwareQuery) -> Vec<&Driver> {
    self
      .by_id
      .values()
      .filter(|driver| selection.matches_driver(driver))
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

#[derive(Debug, Clone)]
pub struct Driver {
  pub id: usize,
  pub name: &'static str,
  pub assignment: IoAddress,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
}

impl Driver {
  pub fn has_tag<T: Tag + 'static>(&self) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).is::<T>())
  }

  pub(crate) fn has_typed_tag(&self, type_id: TypeId) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).type_id() == type_id)
  }
}
