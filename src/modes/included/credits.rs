use crate::prelude::*;

#[derive(Clone)]
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

impl System for Credits {
  fn is_listening(&self) -> bool {
    self.active
  }

  fn on_game_end(&mut self, _ctx: &mut Context, _game: &mut GameState) {
    self.active = true;
  }

  fn on_ball_end(&mut self, ctx: &mut Context, game: &mut GameState) {
    if game.current_player() > 0 || game.current_ball() > 0 {
      self.active = false;
    }
  }

  fn event_switch_closed(
    &mut self,
    switch: &Switch,
    ctx: &mut Context,
    game: Option<&mut GameState>,
  ) {
    if self.coin_switch_names.contains(&switch.name) && self.current_coins < u8::MAX {
      self.current_coins += 1;
      log::debug!(
        "Credits: Coin inserted. Current coins: {}/{}",
        self.current_coins,
        self.coins_per_credit
      );
    } else if switch.name == self.start_switch_name && self.current_coins >= self.coins_per_credit {
      if !ctx.is_game_started() {
        self.current_coins -= self.coins_per_credit;
        ctx.command(StartGame(1));
      } else if ctx.is_game_started() && game.player_count() < self.max_players {
        self.current_coins -= self.coins_per_credit;
        ctx.add_player();
      }
    }
  }
}
