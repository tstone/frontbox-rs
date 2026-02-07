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

  pub fn clear(&mut self) {
    self.internal.clear();
  }

  pub fn get<T: Default + 'static>(&mut self) -> &T {
    if self.internal.get::<T>().is_some() {
      return self.internal.get::<T>().unwrap();
    } else {
      self.internal.insert::<T>(T::default());
      return self.internal.get::<T>().unwrap();
    }
  }

  pub fn get_mut<T: Default + 'static>(&mut self) -> &mut T {
    if self.internal.get::<T>().is_some() {
      return self.internal.get_mut::<T>().unwrap();
    } else {
      self.internal.insert::<T>(T::default());
      return self.internal.get_mut::<T>().unwrap();
    }
  }

  pub fn insert<T: Default + 'static>(&mut self, value: T) {
    self.internal.insert::<T>(value);
  }

  pub fn remove<T: Default + 'static>(&mut self) {
    self.internal.remove::<T>();
  }
}
