use crate::districts::District;
use crate::prelude::*;

pub type AttractMode = SimpledDistrict;

/// Manages a single set of scenes
pub struct SimpledDistrict {
  scene: Scene,
  store: Store,
}

impl SimpledDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>) -> Box<Self> {
    let initial_scene = initial_scene
      .into_iter()
      .map(|system| SystemContainer::new(system))
      .collect();

    Box::new(Self {
      scene: initial_scene,
      store: Store::new(),
    })
  }
}

impl District for SimpledDistrict {
  fn get_current(&self) -> (&Scene, &Store) {
    (&self.scene, &self.store)
  }

  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store) {
    (&mut self.scene, &mut self.store)
  }
}
