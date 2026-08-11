use frontbox::prelude::*;

use crate::GameManager;

pub trait GameManagementExt {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started.
  fn add_player(&self);
  /// Advances the turn to the next player. Typically should only be called once PlayerTurnEnd event has been fired.
  fn advance_turn(&self);
  /// Ends the game. Typically should only be called once GameEnding event has been fired.
  fn end_game(&self);

  fn is_player_addable(&self) -> bool;
  fn is_game_started(&self) -> bool;

  /// Add points to the current player/team
  fn add_points(&self, points: u32);
  /// Set points multiplier for current player/team
  fn set_multiplier(&self, multiplier: f32);
  /// Clear points multiplier for current player/team
  fn clear_multiplier(&self);
}

impl<'a> GameManagementExt for SystemContext<'a> {
  fn add_player(&self) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.add_player(ctx);
    });
  }

  fn add_points(&self, points: u32) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.add_points(points, ctx);
    });
  }

  fn advance_turn(&self) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.advance_turn(ctx);
    });
  }

  fn set_multiplier(&self, multiplier: f32) {
    with_game_manager(self, |manager| {
      manager.set_multiplier(multiplier);
    });
  }

  fn clear_multiplier(&self) {
    with_game_manager(self, |manager| {
      manager.clear_multiplier();
    });
  }

  fn end_game(&self) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.end_game(ctx);
    });
  }

  fn is_game_started(&self) -> bool {
    self
      .get::<GameManager>()
      .map(|manager| manager.is_game_started())
      .unwrap_or(false)
  }

  fn is_player_addable(&self) -> bool {
    self
      .get::<GameManager>()
      .map(|manager| manager.is_player_addable())
      .unwrap_or(false)
  }
}

fn with_game_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut GameManager) -> T) {
  if let Some(mut system) = ctx.get::<GameManager>() {
    f(&mut system);
  } else {
    log::error!("GameManager not running.");
  }
}
