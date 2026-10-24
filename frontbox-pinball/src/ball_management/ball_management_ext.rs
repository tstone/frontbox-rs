use frontbox::prelude::*;

use crate::ball_management::{BallManager, Launch};

pub trait BallManagementExt {
  /// Puts a ball into play. Balls are queued and launched one at a time.
  fn eject_ball(&self, launch: Launch);
  /// Puts `count` balls into play
  fn eject_balls(&self, count: u8, launch: Launch);
  /// True if `Launch::Auto` can launch a ball without the player
  fn can_auto_launch(&self) -> bool;

  /// Balls currently in play, not counting balls that have been temporarily removed from play
  fn balls_in_play(&self) -> u8;
  /// Takes the balls currently present as all of the balls there are, and clears any temporarily removed from play
  fn establish_ball_count(&self);

  /// When a ball is locked, held, or otherwise should no longer be expected to drain
  fn ball_temporarily_removed_from_play(&self);
  /// When a ball is unlocked, released, or otherwise should now be expected to drain
  fn ball_returned_to_play(&self);
}

impl<'a> BallManagementExt for SystemContext<'a> {
  fn eject_ball(&self, launch: Launch) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.eject(launch, ctx.into());
    });
  }

  fn eject_balls(&self, count: u8, launch: Launch) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.eject_many(count, launch, ctx.into());
    });
  }

  fn can_auto_launch(&self) -> bool {
    self
      .get::<BallManager>()
      .map(|manager| manager.can_auto_launch())
      .unwrap_or(false)
  }

  fn balls_in_play(&self) -> u8 {
    self
      .get::<BallManager>()
      .map(|manager| manager.balls_in_play(self.into()))
      .unwrap_or(0)
  }

  fn establish_ball_count(&self) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.establish_ball_count(ctx.into());
    });
  }

  fn ball_temporarily_removed_from_play(&self) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.ball_temporarily_removed_from_play(ctx.into());
    });
  }

  fn ball_returned_to_play(&self) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.ball_returned_to_play(ctx.into());
    });
  }
}

fn with_ball_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut BallManager) -> T) {
  if let Some(mut system) = ctx.get::<BallManager>() {
    f(&mut system);
  } else {
    log::error!("BallManager not running.");
  }
}
