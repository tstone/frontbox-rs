use crate::prelude::*;

#[derive(Debug)]
pub struct PlayerManager {
  ordered_trough_switches: Vec<&'static str>,
  max_balls_in_trough: u8,
}

impl PlayerManager {
  pub fn new(ordered_trough_switches: Vec<&'static str>, max_balls_in_trough: u8) -> Box<Self> {
    Box::new(Self {
      ordered_trough_switches,
      max_balls_in_trough,
    })
  }
}

impl System for PlayerManager {
  fn event_switch_closed(&mut self, switch: &Switch, ctx: &mut Context) {
    let game = ctx.game();
    if game.not_running() {
      return;
    }

    if self.ordered_trough_switches.contains(&switch.name) {
      // Check if all expected trough switches are closed
      let all_closed = self
        .ordered_trough_switches
        .iter()
        .all(|&sw_name| ctx.is_switch_closed(sw_name).unwrap_or(false));

      if all_closed {
        log::debug!("PlayerManager: All balls in trough, starting player change.",);
        ctx.next_player();
      }
    }
  }
}

pub struct PlayerBallInfo {
  pub extra_balls: u8,
}

// TODO: systems should be able to add custom commands
// Player manager should add "next player", "extra ball", etc. commands
