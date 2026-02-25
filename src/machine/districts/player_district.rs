use crate::prelude::*;
use crate::districts::District;

/// A machine district that supports multiple players, each with their own set of scenes.
pub struct PlayerDistrict {
  // The initial scene to use as the basis for each player/team
  initial_scene: Scene,
  /// Active stack, one per player
  player_stacks: Vec<Vec<Scene>>,
  // Store for each player
  player_stores: Vec<Store>,
  /// Index of the current player
  index: u8,
}

impl PlayerDistrict {
  pub fn new(initial_scene: Vec<Box<dyn System>>) -> Box<Self> {
    let mut player_stacks = Vec::new();
    let copy: Vec<SystemContainer> = initial_scene
      .iter()
      .map(|system| SystemContainer::new(dyn_clone::clone_box(&**system)))
      .collect();
    player_stacks.push(vec![copy]);

    let mut player_stores = Vec::new();
    player_stores.push(Store::new());

    let initial_scene = initial_scene
      .into_iter()
      .map(|system| SystemContainer::new(system))
      .collect();

    Box::new(Self {
      initial_scene,
      player_stacks,
      index: 0,
      player_stores,
    })
  }
}

impl District for PlayerDistrict {
  fn get_current(&self) -> (&Scene, &Store) {
    let scene = self
      .player_stacks
      .get(self.index as usize)
      .and_then(|stack| stack.last())
      .unwrap();

    let store = self.player_stores.get(self.index as usize).unwrap();

    (scene, store)
  }

  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store) {
    let scene = self
      .player_stacks
      .get_mut(self.index as usize)
      .and_then(|stack| stack.last_mut())
      .unwrap();

    let store = self.player_stores.get_mut(self.index as usize).unwrap();

    (scene, store)
  }

  fn push_scene(&mut self, scene: Scene) {
    self
      .player_stacks
      .get_mut(self.index as usize)
      .unwrap()
      .push(scene);
  }

  fn pop_scene(&mut self) {
    self
      .player_stacks
      .get_mut(self.index as usize)
      .unwrap()
      .pop();
  }

  fn on_district_enter(&self, ctx: &mut Context) {
    ctx.start_game();
  }

  fn on_add_player(&mut self, _new_player: u8) {
    let copy: Vec<SystemContainer> = self
      .initial_scene
      .iter()
      .map(|system| SystemContainer::new(dyn_clone::clone_box(&**system)))
      .collect();
    self.player_stacks.push(vec![copy]);
    self.player_stores.push(Store::new());
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}
