use crate::prelude::*;

pub struct PlayerPointsPlugin;

impl Plugin for PlayerPointsPlugin {
  fn register(&self, machine: &mut Machine) {
    machine.add_machine_scene(PlayerPointsSystem);
  }
}

#[derive(Debug, Clone)]
pub struct PlayerPointsSystem;

impl System for PlayerPointsSystem {
  fn on_game_start(&mut self, _ctx: &mut Context, game: &mut GameState) {
    game.insert::<PlayerPoints>(PlayerPoints::default());
  }

  fn on_ball_end(&mut self, _ctx: &mut Context, game: &mut GameState) {
    let points = game.get_mut::<PlayerPoints>();
    // machine.display_wait(EndOfBallDisplay {
    //   total_points: points.total + points.current_ball + points.bonus,
    //   ball_points: points.current_ball,
    //   bonus_points: points.bonus,
    // });

    points.total += points.current_ball + points.bonus;
    points.current_ball = 0;
    points.bonus = 0;
  }
}

#[derive(Default)]
pub struct PlayerPoints {
  total: u32,
  current_ball: u32,
  bonus: u32,
}

pub struct AddPoints(u32);

impl Command for AddPoints {
  fn execute(&self, machine: &mut Machine) {
    if let Some(game) = machine.game() {
      let points = game.get_mut::<PlayerPoints>();
      points.current_ball += self.0;
    }
  }
}

pub struct AddBonus(u32);

impl Command for AddBonus {
  fn execute(&self, machine: &mut Machine) {
    if let Some(game) = machine.game() {
      let points = game.get_mut::<PlayerPoints>();
      points.bonus += self.0;
      // TODO: commands are automatically emitted as events?
    }
  }
}
