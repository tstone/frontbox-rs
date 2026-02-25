use crate::districts::District;
use crate::prelude::*;

pub struct TeamDistrictSystem {
  /// Active scene, one per team
  pub(crate) team_scenes: Vec<Scene>,
  /// Index of the current player
  pub(crate) index: u8,
  pub(crate) player_team_map: Vec<u8>,
}

impl SystemDistrict for TeamDistrictSystem {
  fn get_current(&self) -> &Scene {
    self
      .team_scenes
      .get(self.player_team_map[self.index as usize] as usize)
      .expect("Team index out of bounds")
  }

  fn get_current_mut(&mut self) -> &mut Scene {
    self
      .team_scenes
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .expect("Team index out of bounds")
  }

  fn on_district_enter(&self, ctx: &mut Context) {
    ctx.start_game();
  }

  fn on_add_player(&mut self, _new_player: u8) {
    // Nothing happens because each team scene was already created at initialization.
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}

pub struct TeamDistrictStorage {
  // Store for each team
  pub(crate) team_stores: Vec<Store>,
  /// Index of the current player
  pub(crate) index: u8,
  pub(crate) player_team_map: Vec<u8>,
}

impl StorageDistrict for TeamDistrictStorage {
  fn get_current(&self) -> &Store {
    self
      .team_stores
      .get(self.player_team_map[self.index as usize] as usize)
      .expect("Team index out of bounds")
  }

  fn get_current_mut(&mut self) -> &mut Store {
    self
      .team_stores
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .expect("Team index out of bounds")
  }

  fn on_add_player(&mut self, _new_player: u8) {
    // Nothing happens because each team store was already created at initialization.
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}

/// A machine district that supports multiple teams, each with their own set of scenes.
pub struct TeamDistrict {
  scenes: TeamDistrictSystem,
  storage: TeamDistrictStorage,
}

impl TeamDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>, player_team_map: Vec<u8>) -> Box<Self> {
    let mut team_scenes = Vec::new();
    let mut team_stores = Vec::new();
    let team_count = player_team_map.iter().max().unwrap_or(&0) + 1;

    // Create a scene and store for each team
    for _ in 0..team_count {
      let copy: Scene = initial_scene
        .iter()
        .map(|system| SystemContainer::new(dyn_clone::clone_box(&**system)))
        .collect();
      team_scenes.push(copy);
      team_stores.push(Store::new());
    }

    Box::new(Self {
      scenes: TeamDistrictSystem {
        team_scenes,
        index: 0,
        player_team_map: player_team_map.clone(),
      },
      storage: TeamDistrictStorage {
        team_stores,
        index: 0,
        player_team_map,
      },
    })
  }
}

impl District for TeamDistrict {
  fn split(self: Box<Self>) -> (Box<dyn SystemDistrict>, Box<dyn StorageDistrict>) {
    let system_district = Box::new(self.scenes);
    let storage_district = Box::new(self.storage);
    (system_district, storage_district)
  }
}
