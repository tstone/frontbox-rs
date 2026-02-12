use crate::prelude::*;

pub type Scene = Vec<SystemContainer>;

/// A runtime manages which stack of scenes is currently active, acting as a switchboard operator
#[allow(unused)]
pub trait Runtime {
  fn get_current(&self) -> (&Scene, &Store);
  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store);

  fn push_scene(&mut self, scene: Scene);
  fn pop_scene(&mut self);

  fn on_runtime_enter(&self, ctx: &mut Context) {}
  fn on_add_player(&mut self, player_index: u8) {}
  fn on_change_player(&mut self, player_index: u8) {}
  fn on_runtime_exit(&mut self, ctx: &mut Context) {}

  fn get_current_scene(&self) -> &Scene {
    let (scene, _) = self.get_current();
    scene
  }

  fn get_current_scene_mut(&mut self) -> &mut Scene {
    let (scene, _) = self.get_current_mut();
    scene
  }

  fn get_current_store(&self) -> &Store {
    let (_, store) = self.get_current();
    store
  }

  fn get_current_store_mut(&mut self) -> &mut Store {
    let (_, store) = self.get_current_mut();
    store
  }
}
