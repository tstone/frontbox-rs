use crate::prelude::*;
use crate::districts::District;

pub type AttractMode = SimpledDistrict;

/// Manages a single set of scenes
pub struct SimpledDistrict {
  stack: Vec<Scene>,
  store: Store,
}

impl SimpledDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>) -> Box<Self> {
    let initial_scene = initial_scene
      .into_iter()
      .map(|system| SystemContainer::new(system))
      .collect();

    Box::new(Self {
      stack: vec![initial_scene],
      store: Store::new(),
    })
  }
}

impl District for SimpledDistrict {
  fn get_current(&self) -> (&Scene, &Store) {
    (self.stack.last().unwrap(), &self.store)
  }

  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store) {
    (self.stack.last_mut().unwrap(), &mut self.store)
  }

  fn push_scene(&mut self, scene: Scene) {
    self.stack.push(scene);
  }

  fn pop_scene(&mut self) {
    self.stack.pop();
  }
}
