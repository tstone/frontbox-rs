use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct PlayerPointsSystem;

impl System for PlayerPointsSystem {
  fn on_game_start(&mut self, ctx: &mut Context) {
    ctx.insert::<PlayerPoints>(PlayerPoints::default());
  }

  fn on_ball_end(&mut self, ctx: &mut Context) {
    let points = ctx.get_mut::<PlayerPoints>();
    points.total += points.current_ball_points + points.bonus;
    points.current_ball_points = 0;
    points.bonus = 0;
  }
}

#[derive(Default)]
pub struct PlayerPoints {
  total: u32,
  current_ball_points: u32,
  bonus: u32,
}

pub struct AddPoints(pub u32);

impl Command for AddPoints {
  fn execute(&self, machine: &mut Machine) {
    if machine.is_game_started() {
      let points = machine.active_store().get_mut::<PlayerPoints>();
      points.current_ball_points += self.0;
    }
  }
}

pub struct AddBonus(pub u32);

impl Command for AddBonus {
  fn execute(&self, machine: &mut Machine) {
    if machine.is_game_started() {
      let points = machine.active_store().get_mut::<PlayerPoints>();
      points.bonus += self.0;
      // TODO: are commands automatically emitted as events?
    }
  }
}
