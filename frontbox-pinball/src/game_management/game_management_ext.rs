use frontbox_core::prelude::*;

use crate::{GameEndReason, GameManager};

pub trait GameManagementExt {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started.
  fn add_player(&self);
  /// Ends the game in progress early. See `GameManagement::quit_game`.
  fn quit_game(&self, reason: GameEndReason);
  /// The current player tilts. See `GameManagement::tilt`.
  fn tilt(&self);

  /// Add points to the current player/team
  fn add_points(&self, points: u32);
  /// Set points multiplier for current player/team
  fn set_multiplier(&self, multiplier: f32);
  /// Clear points multiplier for current player/team
  fn clear_multiplier(&self);

  /// Give the current player an extra ball. See `GameManagement::grant_extra_ball`.
  fn grant_extra_ball(&self) -> bool;
  /// Extra balls the current player has and hasn't used
  fn extra_balls(&self) -> u8;
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

  fn quit_game(&self, reason: GameEndReason) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.quit_game(reason, ctx.into());
    });
  }

  fn tilt(&self) {
    let ctx = self;
    with_game_manager(self, |manager| {
      manager.tilt(ctx.into());
    });
  }

  fn grant_extra_ball(&self) -> bool {
    let ctx = self;
    with_game_manager(self, |manager| manager.grant_extra_ball(ctx.into())).unwrap_or(false)
  }

  fn extra_balls(&self) -> u8 {
    with_game_manager(self, |manager| {
      manager
        .game_state()
        .map(|state| state.extra_balls(state.current_player()))
        .unwrap_or(0)
    })
    .unwrap_or(0)
  }
}

fn with_game_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut GameManager) -> T) -> Option<T> {
  if let Some(mut system) = ctx.systems.get::<GameManager>() {
    Some(f(&mut system))
  } else {
    log::error!("GameManager not running.");
    None
  }
}
