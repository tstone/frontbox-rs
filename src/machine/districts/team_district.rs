use crate::districts::District;
use crate::prelude::*;

pub struct TeamDistrict {
  /// Active scene, one per team
  team_scenes: Vec<Scene>,
  // Store for each team
  team_stores: Vec<Store>,
  /// Index of the current player
  index: u8,
  player_team_map: Vec<u8>,
}

impl TeamDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>, player_team_map: Vec<u8>) -> Box<Self> {
    let mut team_scenes = Vec::new();
    let mut team_stores = Vec::new();
    let team_count = player_team_map.iter().max().unwrap_or(&0) + 1;

    // Create a stack for each team
    for _ in 0..team_count {
      let copy: Scene = initial_scene
        .iter()
        .map(|system| SystemContainer::new(dyn_clone::clone_box(&**system)))
        .collect();
      team_scenes.push(copy);
      team_stores.push(Store::new());
    }

    Box::new(Self {
      team_scenes,
      team_stores,
      index: 0,
      player_team_map,
    })
  }
}

impl District for TeamDistrict {
  fn get_current(&self) -> (&Scene, &Store) {
    let scene = self
      .team_scenes
      .get(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    let store = self
      .team_stores
      .get(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    (scene, store)
  }

  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store) {
    let scene = self
      .team_scenes
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    let store = self
      .team_stores
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    (scene, store)
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
