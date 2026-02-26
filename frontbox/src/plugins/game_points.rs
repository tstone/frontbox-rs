use crate::prelude::*;

/**
 * GamePoints is an example of a simple way to manage points. It works by defining a shared data structure.
 * It exposes methods on the Context to modify the points, which internally updates the shared data structure.
 * On ball end current points/bonus can be copied to the total points/bonus depending on game rules, tilt, etc.
 *
 * Because this is an extension trait, it can be used by importing it:
 *
 * ```rust
 * use frontbox::plugins::game_points::*;
 * ```
 *
 * In reality most games will probably want to make their own version of this that keeps track of game-specific
 * scoring information, in addition to just points/bonus.
 */
#[derive(Default, Serialize, Storable)]
#[allow(dead_code)]
pub struct GamePoints {
  total_points: u32,
  total_bonus: u32,
  current_ball_points: u32,
  current_ball_bonus: u32,
}

#[allow(dead_code)]
pub trait GamePointsExt {
  fn add_points(&mut self, points: u32);
  fn add_bonus(&mut self, bonus: u32);
  fn merge_ball_points(&mut self);
}

impl GamePointsExt for Context<'_> {
  fn add_points(&mut self, points: u32) {
    self.store().with(move |store| {
      let game_points = store.get_mut::<GamePoints>();
      game_points.current_ball_points += points;
    });
  }

  fn add_bonus(&mut self, bonus: u32) {
    self.store().with(move |store| {
      let game_points = store.get_mut::<GamePoints>();
      game_points.current_ball_bonus += bonus;
    });
  }

  /// Merge points/bonus for current ball into total points/bonus
  fn merge_ball_points(&mut self) {
    self.store().with(move |store| {
      let game_points = store.get_mut::<GamePoints>();
      game_points.total_points += game_points.current_ball_points;
      game_points.total_bonus += game_points.current_ball_bonus;
      game_points.current_ball_points = 0;
      game_points.current_ball_bonus = 0;
    });
  }
}
