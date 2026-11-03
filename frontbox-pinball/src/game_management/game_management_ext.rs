use frontbox::prelude::*;

use crate::GameManager;

pub trait GameManagementExt {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started.
  fn add_player(&self);
  /// Ends the game in progress early. See `GameManagement::quit_game`.
  fn quit_game(&self);

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
      manager.add_player(ctx.into());
    });
  }

  fn add_points(&self, points: u32) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.add_points(points, ctx.into());
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

  fn quit_game(&self) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.quit_game(ctx.into());
    });
  }
}

fn with_game_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut GameManager) -> T) {
  if let Some(mut system) = ctx.get::<GameManager>() {
    f(&mut system);
  } else {
    log::error!("GameManager not running.");
  }
}
