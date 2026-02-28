use crate::districts::District;
use crate::prelude::*;
use crate::systems::SystemContainer;

pub(crate) struct SimpleSystemDistrict {
  pub(crate) scene: Scene,
}

impl SystemDistrict for SimpleSystemDistrict {
  fn get_current(&self) -> &Scene {
    &self.scene
  }

  fn get_current_mut(&mut self) -> &mut Scene {
    &mut self.scene
  }
}

pub(crate) struct SimpleStorageDistrict {
  pub(crate) store: Store,
}

impl StorageDistrict for SimpleStorageDistrict {
  fn get_current(&self) -> &Store {
    &self.store
  }

  fn get_current_mut(&mut self) -> &mut Store {
    &mut self.store
  }
}

/// Manages a single set of scenes
pub struct SimpledDistrict {
  systems: SimpleSystemDistrict,
  storage: SimpleStorageDistrict,
}

impl SimpledDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>) -> Box<Self> {
    let initial_scene = initial_scene
      .into_iter()
      .map(|system| SystemContainer::new(next_listener_id(), system))
      .collect();

    Box::new(Self {
      systems: SimpleSystemDistrict {
        scene: initial_scene,
      },
      storage: SimpleStorageDistrict {
        store: Store::new(),
      },
    })
  }
}

impl District for SimpledDistrict {
  fn split(self: Box<Self>) -> (Box<dyn SystemDistrict>, Box<dyn StorageDistrict>) {
    let system_district = Box::new(self.systems);
    let storage_district = Box::new(self.storage);
    (system_district, storage_district)
  }
}
