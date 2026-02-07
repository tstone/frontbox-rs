use crate::prelude::*;

/// End current system
pub struct EndSelf;

impl Command for EndSelf {
  fn execute(&self, system_id: usize, machine: &mut Machine) {
    machine.terminate_system(system_id);
  }
}

/// End current system and replace with given one
pub struct ReplaceSelf {
  pub new_system: Box<dyn System>,
}

impl Command for ReplaceSelf {
  fn execute(&self, system_id: usize, machine: &mut Machine) {
    machine.replace_system(system_id, dyn_clone::clone_box(&*self.new_system));
  }
}

/// Add a new system to the current scene
pub struct AddSystem {
  pub system: Box<dyn System>,
}

impl Command for AddSystem {
  fn execute(&self, _system_id: usize, machine: &mut Machine) {
    machine.add_system(dyn_clone::clone_box(&*self.system));
  }
}

/// Push a new scene on top of the current one
pub struct PushScene {
  pub scene: Scene,
}

impl Command for PushScene {
  fn execute(&self, _system_id: usize, machine: &mut Machine) {
    let mut scene = Vec::new();
    for system in &self.scene {
      scene.push(dyn_clone::clone_box(&**system));
    }
    machine.push_scene(scene);
  }
}

/// Pop the current scene, returning to the previous one
pub struct PopScene;

impl Command for PopScene {
  fn execute(&self, _system_id: usize, machine: &mut Machine) {
    machine.pop_scene();
  }
}
