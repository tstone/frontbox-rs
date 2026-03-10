use crate::prelude::{StorableType, Store};

pub struct States {
  store: Store,
}

impl States {
  pub fn new() -> Self {
    Self {
      store: Store::new(),
    }
  }

  pub fn is<T: StorableType>(&self, value: T) -> bool {
    self.store.get::<T>().map_or(false, |v| *v == value)
  }

  pub fn is_not<T: StorableType>(&self, value: T) -> bool {
    !self.is(value)
  }

  pub(crate) fn set<T: StorableType>(&mut self, value: T) {
    self.store.insert(value);
  }
}
