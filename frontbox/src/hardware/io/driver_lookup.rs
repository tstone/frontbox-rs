use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DriverLookup {
  by_id: HashMap<usize, Driver>,
  by_name: HashMap<&'static str, Driver>,
  configs: HashMap<usize, DriverMode>,
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

      if let Some(config) = &addressed.definition.mode {
        configs.insert(addressed.id, config.clone());
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

  pub fn by_name(&self, driver_name: &str) -> Option<&Driver> {
    self.by_name.get(driver_name)
  }

  pub fn by_name_mut(&mut self, driver_name: &str) -> Option<&mut Driver> {
    self.by_name.get_mut(driver_name)
  }

  pub fn config(&self, name: &str) -> Option<&DriverMode> {
    self
      .by_name
      .get(name)
      .and_then(|driver| self.configs.get(&driver.id))
  }

  /// (driver name, switch name) for every switch a driver mode references that isn't in `switches`
  pub fn unknown_switch_references(
    &self,
    switches: &SwitchLookup,
  ) -> Vec<(&'static str, &'static str)> {
    let mut unknown = Vec::new();
    for (id, mode) in &self.configs {
      let driver = self.by_id[id].name;
      for switch in mode.switch_names() {
        if switches.by_name(switch).is_none() {
          unknown.push((driver, switch));
        }
      }
    }
    unknown.sort();
    unknown
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

  pub fn query_iter<'a>(&'a self, query: &DriverQ) -> impl Iterator<Item = &'a Driver> {
    self.by_id.values().filter(|driver| query.matches(driver))
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

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Driver {
  pub id: usize,
  pub name: &'static str,
  pub assignment: IoAddress,
  #[cfg_attr(feature = "ts", ts(type = "Array<string>"))]
  pub tags: Vec<Box<dyn Tag>>,
  #[cfg_attr(feature = "ts", ts(type = "[number, number, number] | null"))]
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

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use super::*;

  static SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new("real").build());
  static GOOD: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("good")
      .mode(DriverMode::pulse().trigger_mode(DriverTriggerMode::Switch("real")).build())
      .build()
  });
  static TYPO: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("typo")
      .mode(DriverMode::pulse().trigger_mode(DriverTriggerMode::Switch("reel")).build())
      .build()
  });
  static FLIPPER: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("flipper")
      .mode(DriverMode::flipper_main_direct("real", "missing_eos").build())
      .build()
  });
  static VIRTUAL: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("virtual")
      .mode(DriverMode::pulse().build())
      .build()
  });

  fn addressed<T: HardwareDefinition>(defs: Vec<&'static T>) -> Vec<IoAddressed<T>> {
    defs
      .into_iter()
      .enumerate()
      .map(|(id, definition)| IoAddressed {
        definition,
        assignment: IoAddress::new(0, id as u16),
        id,
      })
      .collect()
  }

  #[test]
  fn unknown_switch_references() {
    let switches = SwitchLookup::new(addressed(vec![&*SWITCH]), vec![]);
    let drivers = DriverLookup::new(addressed(vec![&*GOOD, &*TYPO, &*FLIPPER, &*VIRTUAL]));

    assert_eq!(
      drivers.unknown_switch_references(&switches),
      vec![("flipper", "missing_eos"), ("typo", "reel")]
    );
  }
}
