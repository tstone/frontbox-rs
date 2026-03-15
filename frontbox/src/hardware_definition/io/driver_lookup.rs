use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;
use serde::Serialize;

#[derive(Debug, Serialize, Storable)]
pub struct DriverLookup {
  by_id: HashMap<usize, Driver>,
  by_name: HashMap<&'static str, Driver>,
}

impl DriverLookup {
  pub fn new(drivers: Vec<Driver>) -> Self {
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();

    for driver in drivers {
      by_id.insert(
        driver.id,
        Driver {
          id: driver.id,
          name: driver.name,
          native: driver.native.clone(),
          config: driver.config.clone(),
        },
      );

      by_name.insert(
        driver.name,
        Driver {
          id: driver.id,
          name: driver.name,
          native: driver.native.clone(),
          config: driver.config.clone(),
        },
      );
    }

    Self { by_id, by_name }
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

#[derive(Debug, Clone, Serialize)]
pub struct Driver {
  pub id: usize,
  pub name: &'static str,
  pub native: NativeIdentity,
  pub config: Option<DriverConfig>,
}
