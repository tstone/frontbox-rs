use crate::plugins::trough::{BallEnteredTrough, BallExitedTrough};
pub use crate::prelude::*;

pub struct TroughSystem {
  trough_switches: Vec<&'static str>,
}

impl TroughSystem {
  /// # Arguments
  /// * `switches` - List of trough switches, in order. Index 0 is the switch nearest the exit.
  pub fn new(switches: Vec<&'static str>) -> Self {
    Self {
      trough_switches: switches,
    }
  }

  fn get_occupancy(&self, ctx: &Context_OLD) -> Vec<bool> {
    self
      .trough_switches
      .iter()
      .map(|name| ctx.is_switch_closed(name).unwrap_or(false))
      .collect()
  }
}

impl System for TroughSystem {
  fn on_event(&mut self, event: &dyn Event, _ctx: &Context_OLD, cmds: &mut Commands) {
    handle_event!(event, {
      SwitchClosed => |e| {
        if self.trough_switches.iter().last().map(|s| *s == e.switch.name).unwrap_or(false) {
          cmds.emit(BallEnteredTrough::new(self.get_occupancy(_ctx)));
        }
      },
      SwitchOpened => |e| {
        if self.trough_switches.iter().first().map(|s| *s == e.switch.name).unwrap_or(false) {
          cmds.emit(BallExitedTrough::new(self.get_occupancy(_ctx)));
        }
      }
    })
  }
}
