use crate::prelude::*;
use crate::runtimes::Runtime;

pub type AttractMode = SimpleRuntime;

/// Manages a single stack of scenes with no additional behavior or stack switching
pub struct SimpleRuntime {
  stack: Vec<Scene>,
  store: Store,
}

impl SimpleRuntime {
  pub fn new(initial_scene: Scene) -> Box<Self> {
    Box::new(Self {
      stack: vec![initial_scene],
      store: Store::new(),
    })
  }
}

impl Runtime for SimpleRuntime {
  fn get_current(&mut self) -> (&mut Scene, &mut Store) {
    (self.stack.last_mut().unwrap(), &mut self.store)
  }

  fn push_scene(&mut self, scene: Scene) {
    self.stack.push(scene);
  }

  fn pop_scene(&mut self) {
    self.stack.pop();
  }
}
