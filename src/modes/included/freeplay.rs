use crate::modes::prelude::*;

/// Starts a new game when the start button is pressed. No credits required.
#[derive(Debug)]
pub struct Freeplay {
  active: bool,
  start_switch_name: &'static str,
  max_players: u8,
}

impl Freeplay {
  pub fn new(start_switch_name: &'static str, max_players: u8) -> Box<Self> {
    Box::new(Self {
      active: true,
      start_switch_name,
      max_players,
    })
  }
}

impl MachineMode for Freeplay {
  fn is_listening(&self) -> bool {
    self.active
  }

  fn on_game_state_changed(
    &mut self,
    _old: &GameState,
    new: &GameState,
    _switches: &SwitchContext,
  ) {
    self.active =
      !new.is_started() || (new.current_player() == Some(0) && new.current_ball() == Some(0))
  }

  fn event_switch_closed(&mut self, switch: &Switch, ctx: &mut MachineContext) {
    if switch.name == self.start_switch_name {
      if !ctx.game().is_started() {
        ctx.start_game();
      } else if ctx.game().player_count() < self.max_players {
        ctx.add_player();
      }
    }
  }
}
