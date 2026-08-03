use std::any::type_name;
use std::cell::RefMut;

use crate::prelude::System;
use crate::prelude::system_collection::SystemCollection;

#[derive(Clone)]
pub struct SystemsContext<'a> {
  pub(crate) system_collection: &'a SystemCollection,
  pub(crate) system_id: u64,
}

impl<'a> SystemsContext<'a> {
  /// Lookup a global system or sibling system by type
  pub fn get<T: System + 'static>(&'_ self) -> Option<RefMut<'_, T>> {
    // Priority is given to nearness: siblings are searched first, then global
    if let Some(parent) = self.system_collection.parent(&self.system_id)
      && let Some(system) = parent.get_by_type::<T>()
    {
      Some(system)
    } else if let Some(system) = self.system_collection.systems.get_by_type::<T>() {
      Some(system)
    } else {
      None
    }
  }

  /// Lookup a global system or sibling system by type, panics if does not exist
  pub fn expect<T: System + 'static>(&'_ self) -> RefMut<'_, T> {
    let system_name = type_name::<T>();
    self.get::<T>().expect(
      format!(
        "Expected system {} was not found. Make sure it was added to the App.",
        system_name
      )
      .as_str(),
    )
  }
}
