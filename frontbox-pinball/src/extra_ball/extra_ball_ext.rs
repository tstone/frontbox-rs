use frontbox::prelude::*;

use crate::ExtraBallSystem;

pub trait ExtraBallExt {
  /// Give the player an extra ball, used the next time their last ball drains
  fn grant_extra_ball(&self);
  /// Extra balls the player has been granted and not yet used
  fn extra_balls(&self) -> u8;
}

impl<'a> ExtraBallExt for SystemContext<'a> {
  fn grant_extra_ball(&self) {
    if let Some(mut extra_ball) = self.get::<ExtraBallSystem>() {
      extra_ball.grant(self.into());
    } else {
      log::warn!(target: "frontbox::extraball", "ExtraBallSystem not running.");
    }
  }

  fn extra_balls(&self) -> u8 {
    self
      .get::<ExtraBallSystem>()
      .map(|extra_ball| extra_ball.count())
      .unwrap_or(0)
  }
}
