use crate::districts::District;
use crate::prelude::*;
use crate::systems::SystemContainer;

pub struct PlayerDistrictSystem {
  // The initial scene to use as the basis for each player/team
  pub(crate) initial_scene: Scene,
  /// Active stack, one per player
  pub(crate) player_scenes: Vec<Scene>,
  /// Index of the current player
  pub(crate) index: u8,
}

impl SystemDistrict for PlayerDistrictSystem {
  fn get_current(&self) -> &Scene {
    self
      .player_scenes
      .get(self.index as usize)
      .expect("Player index out of bounds")
  }

  fn get_current_mut(&mut self) -> &mut Scene {
    self
      .player_scenes
      .get_mut(self.index as usize)
      .expect("Player index out of bounds")
  }

  fn on_district_enter(&self, ctx: &mut Context) {
    ctx.start_game();
  }

  fn on_add_player(&mut self, _new_player: u8) {
    let copy: Scene = self
      .initial_scene
      .iter()
      .map(|system| SystemContainer::new(next_listener_id(), dyn_clone::clone_box(&**system)))
      .collect();
    self.player_scenes.push(copy);
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}

pub struct PlayerDistrictStorage {
  // Store for each player
  pub(crate) player_stores: Vec<Store>,
  /// Index of the current player
  pub(crate) index: u8,
}

impl StorageDistrict for PlayerDistrictStorage {
  fn get_current(&self) -> &Store {
    self
      .player_stores
      .get(self.index as usize)
      .expect("Player index out of bounds")
  }

  fn get_current_mut(&mut self) -> &mut Store {
    self
      .player_stores
      .get_mut(self.index as usize)
      .expect("Player index out of bounds")
  }

  fn on_add_player(&mut self, _new_player: u8) {
    self.player_stores.push(Store::new());
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}

/// A machine district that supports multiple players, each with their own set of scenes.
pub struct PlayerDistrict {
  scenes: PlayerDistrictSystem,
  storage: PlayerDistrictStorage,
}

impl PlayerDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>) -> Box<Self> {
    let mut player_scenes = Vec::new();
    let copy: Scene = initial_scene
      .iter()
      .map(|system| SystemContainer::new(next_listener_id(), dyn_clone::clone_box(&**system)))
      .collect();
    player_scenes.push(copy);

    let mut player_stores = Vec::new();
    player_stores.push(Store::new());

    let initial_scene = initial_scene
      .into_iter()
      .map(|system| SystemContainer::new(next_listener_id(), system))
      .collect();

    Box::new(Self {
      scenes: PlayerDistrictSystem {
        initial_scene,
        player_scenes,
        index: 0,
      },
      storage: PlayerDistrictStorage {
        player_stores,
        index: 0,
      },
    })
  }
}

impl District for PlayerDistrict {
  fn split(self: Box<Self>) -> (Box<dyn SystemDistrict>, Box<dyn StorageDistrict>) {
    let system_district = Box::new(self.scenes);
    let storage_district = Box::new(self.storage);
    (system_district, storage_district)
  }
}
