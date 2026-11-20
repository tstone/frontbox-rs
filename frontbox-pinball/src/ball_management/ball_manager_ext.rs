use std::cell::RefMut;

use frontbox_core::prelude::*;

use crate::ball_management::{BallManager, Launch};

pub trait BallManagerExt {
  /// The running ball management, for calls not covered below. `None` if it isn't running, or is in use.
  fn ball_manager(&self) -> Option<RefMut<'_, dyn BallManager>>;

  /// Puts a ball into play. Balls are queued and launched one at a time. Returns the id `EjectCompleted` will carry,
  /// or `None` if ball management isn't running.
  fn eject_ball(&self, launch: Launch) -> Option<u64>;
  /// Puts `count` balls into play. Returns the id `EjectCompleted` will carry once they all are, or `None` if ball
  /// management isn't running.
  fn eject_balls(&self, count: u8, launch: Launch) -> Option<u64>;
  /// True if `Launch::Auto` can launch a ball without the player
  fn can_auto_launch(&self) -> bool;
  /// Drops queued balls, completes open requests, and keeps a ball resting in the plunge lane for the next request
  fn cancel_ejects(&self);

  /// Balls currently in play, including balls resting in holders but not locked balls
  fn balls_in_play(&self) -> u8;
  /// Takes the balls currently present (in the trough, the plunge lane, holders and locks) as all of the balls there
  /// are
  fn establish_ball_count(&self);

  /// Reports how many balls are resting in this system right now (a scoop, a VUK), from its own switches. They stay
  /// in play. Report every change, including back to 0.
  fn set_balls_held(&self, count: u8);
  /// Reports how many balls are locked in this system right now (a ball lock), from its own switches. They're out of
  /// play until released. Report every change, including back to 0.
  fn set_balls_locked(&self, count: u8);
}

impl<'a> BallManagerExt for SystemContext<'a> {
  fn ball_manager(&self) -> Option<RefMut<'_, dyn BallManager>> {
    self.systems.get_dyn::<dyn BallManager>()
  }

  fn eject_ball(&self, launch: Launch) -> Option<u64> {
    let ctx = self;
    with_ball_manager(self, |manager| manager.eject(launch, ctx.into()))
  }

  fn eject_balls(&self, count: u8, launch: Launch) -> Option<u64> {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.eject_many(count, launch, ctx.into())
    })
  }

  fn can_auto_launch(&self) -> bool {
    self
      .ball_manager()
      .map(|manager| manager.can_auto_launch())
      .unwrap_or(false)
  }

  fn cancel_ejects(&self) {
    let ctx = self;
    with_ball_manager(self, |manager| manager.cancel_ejects(ctx.into()));
  }

  fn balls_in_play(&self) -> u8 {
    self
      .ball_manager()
      .map(|manager| manager.balls_in_play(self.into()))
      .unwrap_or(0)
  }

  fn establish_ball_count(&self) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.establish_ball_count(ctx.into());
    });
  }

  fn set_balls_held(&self, count: u8) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.set_balls_held(ctx.id(), count, ctx.into());
    });
  }

  fn set_balls_locked(&self, count: u8) {
    let ctx = self;
    with_ball_manager(self, |manager| {
      manager.set_balls_locked(ctx.id(), count, ctx.into());
    });
  }
}

fn with_ball_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut dyn BallManager) -> T) -> Option<T> {
  if let Some(mut system) = ctx.ball_manager() {
    Some(f(&mut *system))
  } else {
    log::error!(target: "frontbox::ball_management", "Ball management is not running.");
    None
  }
}
