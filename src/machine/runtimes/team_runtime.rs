use crate::prelude::*;
use crate::runtimes::Runtime;

pub struct TeamRuntime {
  /// Active stack, one per team
  team_stacks: Vec<Vec<Scene>>,
  /// Index of the current player
  index: u8,
  player_team_map: Vec<u8>,
}

impl TeamRuntime {
  pub fn new(initial_scene: Scene, player_team_map: Vec<u8>) -> Box<Self> {
    let mut team_stacks = Vec::new();
    let team_count = player_team_map.iter().max().unwrap_or(&0) + 1;

    // Create a stack for each team
    for _ in 0..team_count {
      let copy: Vec<Box<dyn System>> = initial_scene
        .iter()
        .map(|system| dyn_clone::clone_box(&**system))
        .collect();
      team_stacks.push(vec![copy]);
    }

    Box::new(Self {
      team_stacks,
      index: 0,
      player_team_map,
    })
  }
}

impl Runtime for TeamRuntime {
  fn current_scene(&mut self) -> &mut Scene {
    self
      .team_stacks
      // get for current team
      .get_mut(self.player_team_map[self.index as usize] as usize)
      // get top of stack
      .and_then(|stack| stack.last_mut())
      .unwrap()
  }

  fn on_add_player(&mut self, _new_player: u8) {
    // Nothing happens because each team stack was already created at initialization.
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}
