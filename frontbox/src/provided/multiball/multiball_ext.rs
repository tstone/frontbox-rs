use crate::prelude::*;
use crate::provided::MultiballSystem;

pub trait MultiballExt {
  fn multiball_add_balls(&self, additional_ball_count: u8);
}

impl<'a> MultiballExt for SystemContext<'a> {
  fn multiball_add_balls(&self, additional_ball_count: u8) {
    if let Some(mut multiball) = self.get::<MultiballSystem>() {
      multiball.add_balls(additional_ball_count, self.into());
    }
  }
}
