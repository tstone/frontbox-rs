use std::ops::{Deref, DerefMut};

use frontbox::{delegate_system, prelude::*};

use crate::ball_management::{TroughBallManagement, TroughManagementConfig};

/// Tracks the balls in play and puts new ones into play.
///
/// ## Events
/// - `BallsDrained` - Emitted when one or more balls leave play
/// - `MaxBallsInPlayChanged` - Emitted when the number of balls that could be in play changes (e.g. a ball is locked)
pub trait BallManagement: SpawnableSystem {
  /// Puts a ball into play. Queued if a ball can't be ejected right now.
  fn eject(&mut self, ctx: &ServiceContext);
  /// Puts `count` balls into play
  fn eject_many(&mut self, count: u8, ctx: &ServiceContext);

  /// Balls currently in play, not counting balls that have been temporarily removed from play
  fn balls_in_play(&self, ctx: &ServiceContext) -> u8;
  /// Takes the balls currently present as all of the balls there are, and clears any temporarily removed from play.
  /// Typically called at the start of a game, so that a ball lost in a previous game isn't waited on forever.
  fn establish_ball_count(&mut self, ctx: &ServiceContext);

  /// When a ball is locked, held, or otherwise should no longer be expected to drain
  fn ball_temporarily_removed_from_play(&mut self, ctx: &ServiceContext);
  /// When a ball is unlocked, released, or otherwise should now be expected to drain
  fn ball_returned_to_play(&mut self, ctx: &ServiceContext);
}

pub struct BallManager {
  inner: Box<dyn BallManagement>,
}

impl BallManager {
  pub fn new(ball_management: impl BallManagement + 'static) -> Self {
    Self {
      inner: Box::new(ball_management),
    }
  }

  pub fn trough(config: TroughManagementConfig) -> Self {
    Self {
      inner: Box::new(TroughBallManagement::new(config)),
    }
  }
}

delegate_system!(BallManager, inner);

impl Deref for BallManager {
  type Target = dyn BallManagement;

  fn deref(&self) -> &Self::Target {
    self.inner.as_ref()
  }
}

impl DerefMut for BallManager {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.as_mut()
  }
}
