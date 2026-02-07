use crate::prelude::*;
use crate::runtimes::Runtime;

pub type AttractMode = SimpleRuntime;

/// Manages a single stack of scenes with no additional behavior or stack switching
pub struct SimpleRuntime {
  stack: Vec<Scene>,
}

impl SimpleRuntime {
  pub fn new(initial_scene: Scene) -> Box<Self> {
    Box::new(Self {
      stack: vec![initial_scene],
    })
  }
}

impl Runtime for SimpleRuntime {
  fn current_scene(&mut self) -> &mut Scene {
    self.stack.last_mut().unwrap()
  }
}
