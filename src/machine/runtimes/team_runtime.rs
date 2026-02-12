use crate::prelude::*;
use crate::runtimes::Runtime;

pub struct TeamRuntime {
  /// Active stack, one per team
  team_stacks: Vec<Vec<Scene>>,
  // Store for each team
  team_stores: Vec<Store>,
  /// Index of the current player
  index: u8,
  player_team_map: Vec<u8>,
}

impl TeamRuntime {
  pub fn new(initial_scene: Vec<Box<dyn System>>, player_team_map: Vec<u8>) -> Box<Self> {
    let mut team_stacks = Vec::new();
    let mut team_stores = Vec::new();
    let team_count = player_team_map.iter().max().unwrap_or(&0) + 1;

    // Create a stack for each team
    for _ in 0..team_count {
      let copy: Vec<SystemContainer> = initial_scene
        .iter()
        .map(|system| SystemContainer::new(dyn_clone::clone_box(&**system)))
        .collect();
      team_stacks.push(vec![copy]);
      team_stores.push(Store::new());
    }

    Box::new(Self {
      team_stacks,
      team_stores,
      index: 0,
      player_team_map,
    })
  }
}

impl Runtime for TeamRuntime {
  fn get_current(&self) -> (&Scene, &Store) {
    let scene = self
      .team_stacks
      .get(self.player_team_map[self.index as usize] as usize)
      .and_then(|stack| stack.last())
      .unwrap();

    let store = self
      .team_stores
      .get(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    (scene, store)
  }

  fn get_current_mut(&mut self) -> (&mut Scene, &mut Store) {
    let scene = self
      .team_stacks
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .and_then(|stack| stack.last_mut())
      .unwrap();

    let store = self
      .team_stores
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .unwrap();

    (scene, store)
  }

  fn push_scene(&mut self, scene: Scene) {
    self
      .team_stacks
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .unwrap()
      .push(scene);
  }

  fn pop_scene(&mut self) {
    self
      .team_stacks
      .get_mut(self.player_team_map[self.index as usize] as usize)
      .unwrap()
      .pop();
  }

  fn on_runtime_enter(&self, ctx: &mut Context) {
    ctx.start_game();
  }

  fn on_add_player(&mut self, _new_player: u8) {
    // Nothing happens because each team stack was already created at initialization.
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}
