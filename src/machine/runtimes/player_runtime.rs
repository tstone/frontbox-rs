use crate::prelude::*;
use crate::runtimes::Runtime;

/// A machine runtime that supports multiple players, each with their own stack of scenes.
pub struct PlayerRuntime {
  // The initial scene to use as the basis for each player/team
  initial_scene: Scene,
  /// Active stack, one per player
  player_stacks: Vec<Vec<Scene>>,
  /// Index of the current player
  index: u8,
}

impl PlayerRuntime {
  pub fn new(initial_scene: Scene) -> Box<Self> {
    let mut player_stacks = Vec::new();

    let copy: Vec<Box<dyn System>> = initial_scene
      .iter()
      .map(|system| dyn_clone::clone_box(&**system))
      .collect();
    player_stacks.push(vec![copy]);

    Box::new(Self {
      initial_scene,
      player_stacks,
      index: 0,
    })
  }
}

impl Runtime for PlayerRuntime {
  fn current_scene(&mut self) -> &mut Scene {
    self
      .player_stacks
      // get for current player
      .get_mut(self.index as usize)
      // get top of stack
      .and_then(|stack| stack.last_mut())
      .unwrap()
  }

  fn on_add_player(&mut self, _new_player: u8) {
    let copy: Vec<Box<dyn System>> = self
      .initial_scene
      .iter()
      .map(|system| dyn_clone::clone_box(&**system))
      .collect();
    self.player_stacks.push(vec![copy]);
  }

  fn on_change_player(&mut self, new_player: u8) {
    self.index = new_player;
  }
}
