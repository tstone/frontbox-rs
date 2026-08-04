use std::any::type_name;
use std::cell::RefMut;

use crate::prelude::{Groups, ROOT_GROUP, System};

#[derive(Clone)]
pub struct SystemsContext<'a> {
  pub(crate) groups: &'a Groups,
  pub(crate) parent_key: &'static str,
}

impl<'a> SystemsContext<'a> {
  /// Lookup a global system or sibling system by type
  pub fn get<T: System + 'static>(&self) -> Option<RefMut<'_, T>> {
    // Priority is given to nearness: siblings are searched first, then global
    if let Some(found_system) = self.search_group(self.parent_key) {
      Some(found_system)
    } else if self.parent_key != ROOT_GROUP
      && let Some(found_system) = self.search_group(ROOT_GROUP)
    {
      Some(found_system)
    } else {
      None
    }
  }

  fn search_group<T: System + 'static>(&self, key: &'static str) -> Option<RefMut<'_, T>> {
    if let Some(parent) = self.groups.get(key)
      && let Some(system) = parent.get_by_type::<T>()
    {
      Some(system)
    } else {
      None
    }
  }

  /// Lookup a global system or sibling system by type, panics if does not exist
  pub fn expect<T: System + 'static>(&self) -> RefMut<'_, T> {
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
