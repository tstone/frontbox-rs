use crate::prelude::*;

#[derive(Debug)]
pub struct Credits {
  active: bool,
  start_switch_name: &'static str,
  coin_switch_names: Vec<&'static str>,
  max_players: u8,
  coins_per_credit: u8,
  current_coins: u8,
}

impl Credits {
  pub fn new(
    start_switch_name: &'static str,
    coin_switch_names: Vec<&'static str>,
    max_players: u8,
    coins_per_credit: u8,
  ) -> Box<Self> {
    Box::new(Self {
      active: false,
      start_switch_name,
      coin_switch_names,
      max_players,
      coins_per_credit,
      current_coins: 0,
    })
  }
}

impl MachineMode for Credits {
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
    if self.coin_switch_names.contains(&switch.name) && self.current_coins < u8::MAX {
      self.current_coins += 1;
      log::debug!(
        "Credits: Coin inserted. Current coins: {}/{}",
        self.current_coins,
        self.coins_per_credit
      );
    } else if switch.name == self.start_switch_name && self.current_coins >= self.coins_per_credit {
      let game = ctx.game();
      if !game.is_started() {
        self.current_coins -= self.coins_per_credit;
        ctx.start_game();
      } else if game.is_started() && game.player_count() < self.max_players {
        self.current_coins -= self.coins_per_credit;
        ctx.add_player();
      }
    }
  }
}
