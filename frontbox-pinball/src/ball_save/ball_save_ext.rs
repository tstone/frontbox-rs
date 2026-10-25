use frontbox::prelude::*;

use crate::BallSaveSystem;

pub trait BallSaveExt {
  /// Save balls for `duration`, replacing any request this system already made
  fn request_ball_save(&self, duration: Duration);
  /// Save balls until `release_ball_save`, replacing any request this system already made
  fn request_ball_save_until_released(&self);
  /// Remove this system's request, if any
  fn release_ball_save(&self);
  /// True while drains are being saved, including the grace period
  fn is_ball_save_active(&self) -> bool;
}

impl<'a> BallSaveExt for SystemContext<'a> {
  fn request_ball_save(&self, duration: Duration) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| {
      ball_save.request(id, duration, self.into())
    });
  }

  fn request_ball_save_until_released(&self) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| {
      ball_save.request_until_released(id, self.into())
    });
  }

  fn release_ball_save(&self) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| ball_save.release(id, self.into()));
  }

  fn is_ball_save_active(&self) -> bool {
    self
      .get::<BallSaveSystem>()
      .map(|ball_save| ball_save.is_saving())
      .unwrap_or(false)
  }
}

fn with_ball_save(ctx: &SystemContext, f: impl FnOnce(&mut BallSaveSystem)) {
  if let Some(mut system) = ctx.get::<BallSaveSystem>() {
    f(&mut system);
  } else {
    log::warn!(target: "frontbox::ballsave", "BallSaveSystem not running.");
  }
}
