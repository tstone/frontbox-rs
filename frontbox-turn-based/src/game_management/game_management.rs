use std::ops::{Deref, DerefMut};

use frontbox::{delegate_system, prelude::*};

use crate::{CompetitiveGame, GameState};

pub trait GameManagement: SpawnableSystem {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started.
  fn add_player(&mut self, ctx: &Context, systems: &Systems);
  /// Advances the turn to the next player. Typically should only be called once PlayerTurnEnd event has been fired.
  fn advance_turn(&mut self, ctx: &Context, systems: &Systems);
  /// Ends the game. Typically should only be called once GameEnding event has been fired.
  fn end_game(&mut self, ctx: &Context);

  fn is_player_addable(&self) -> bool;
  fn is_game_started(&self) -> bool;

  fn game_state(&self) -> Option<&GameState>;

  /// Add points to the current player/team
  fn add_points(&mut self, points: u32, ctx: &mut Context);
  /// Set points multiplier for current player/team
  fn set_multiplier(&mut self, multiplier: f32);
  /// Clear points multiplier for current player/team
  fn clear_multiplier(&mut self);
}

pub struct GameManager {
  inner: Box<dyn GameManagement>,
}

impl GameManager {
  fn new(impl_box: impl GameManagement + 'static) -> Self {
    Self {
      inner: Box::new(impl_box),
    }
  }

  pub fn competitive(
    max_players: u8,
    ball_in_play_switches: Vec<&'static str>,
    player_template: Vec<ChildSystemContainer>,
  ) -> Self {
    Self::new(CompetitiveGame::new(
      max_players,
      ball_in_play_switches,
      player_template,
    ))
  }
}

delegate_system!(GameManager, inner);

impl Deref for GameManager {
  type Target = dyn GameManagement;

  fn deref(&self) -> &Self::Target {
    self.inner.as_ref()
  }
}

impl DerefMut for GameManager {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.as_mut()
  }
}
