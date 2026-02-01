use anymap2::AnyMap;

#[derive(Debug)]
pub struct Store {
  internal: AnyMap,
}

impl Store {
  pub fn new() -> Self {
    Self {
      internal: AnyMap::new(),
    }
  }

  pub fn get_state<T: Default + 'static>(&mut self) -> &T {
    if self.internal.get::<T>().is_some() {
      return self.internal.get::<T>().unwrap();
    } else {
      self.internal.insert::<T>(T::default());
      return self.internal.get::<T>().unwrap();
    }
  }

  pub fn get_state_mut<T: Default + 'static>(&mut self) -> &mut T {
    if self.internal.get::<T>().is_some() {
      return self.internal.get_mut::<T>().unwrap();
    } else {
      self.internal.insert::<T>(T::default());
      return self.internal.get_mut::<T>().unwrap();
    }
  }

  pub fn insert_state<T: Default + 'static>(&mut self, value: T) {
    self.internal.insert::<T>(value);
  }
}
