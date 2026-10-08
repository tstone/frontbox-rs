use frontbox::prelude::*;

use crate::GameStarted;
use crate::ball_management::ball_management_config::BallManagementConfig;
use crate::ball_management::trough::{TroughOccupancyChanged, TroughSystem};

pub struct BallManagementSystem {
  handle: SystemHandle,
  config: BallManagementConfig,
  max_balls_in_play: u8,
  locked_balls: u8,
}

impl BallManagementSystem {
  pub fn new(config: BallManagementConfig) -> Self {
    BallManagementSystem {
      handle: SystemHandle::default(),
      max_balls_in_play: config.trough_switch_names.len() as u8,
      config,
      locked_balls: 0,
    }
  }

  pub fn eject(&mut self, ctx: &ServiceContext) {
    ctx.expect::<TroughSystem>(self.handle).eject(ctx);
  }

  pub fn balls_in_play(&self, ctx: &ServiceContext) -> u8 {
    self.in_play(ctx.expect::<TroughSystem>(self.handle).occupancy())
  }

  fn in_play(&self, occupancy: u8) -> u8 {
    self
      .max_balls_in_play
      .saturating_sub(occupancy)
      .saturating_sub(self.locked_balls)
  }

  /// When a ball is locked, held, or otherwise should no longer be expected in the trough
  pub fn ball_temporarily_removed_from_play(&mut self, ctx: &ServiceContext) {
    let prior = self.locked_balls.clone();
    self.locked_balls = self.locked_balls.saturating_add(1);

    if self.locked_balls < prior {
      ctx.for_system(self.handle).emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: -1,
      });
    }
  }

  /// When a ball is unlocked, released, or otherwise should now be expected in the trough
  pub fn ball_returned_to_play(&mut self, ctx: &ServiceContext) {
    let prior = self.locked_balls.clone();
    self.locked_balls = self.locked_balls.saturating_sub(1);

    if prior > self.locked_balls {
      ctx.for_system(self.handle).emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: 1,
      });
    }
  }

  fn establish_drained_occupancy(&mut self, ctx: &SystemContext) {
    self.max_balls_in_play = ctx.expect::<TroughSystem>().occupancy();
  }

  fn max_in_play(&self) -> u8 {
    self.max_balls_in_play.saturating_sub(self.locked_balls)
  }

  /// If somehow the trough reports more balls that we were expecting
  /// attempt to figure out where it came from and account for it
  fn reconcile_appearing_ball(&mut self, delta: u8, ctx: &SystemContext) {
    let mut rem = delta;

    // first reduce it from locked balls
    loop {
      if rem > 0 && self.locked_balls > 0 {
        self.ball_returned_to_play(ctx.into());
        rem -= 1;
      } else {
        break;
      }
    }

    // then increment max balls
    if rem > 0 {
      self.max_balls_in_play += rem;
    }
  }
}

impl System for BallManagementSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.spawn_system(TroughSystem::new(self.config.clone()));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<GameStarted>() {
      // at the start of a game, use the actual amount of balls present as the target
      self.establish_drained_occupancy(ctx);
      self.locked_balls = 0;
    } else if let Some(TroughOccupancyChanged {
      current_occupancy,
      entered,
      ..
    }) = event.downcast_ref::<TroughOccupancyChanged>()
      && *entered > 0
    {
      // Balls beyond what could be in the trough weren't counted as in play; account for them rather than call them drains
      let excess = current_occupancy
        .saturating_sub(self.max_in_play())
        .min(*entered);
      if excess > 0 {
        self.reconcile_appearing_ball(excess, ctx);
      }

      let drained = entered - excess;
      if drained > 0 {
        ctx.emit(BallsDrained {
          count: drained,
          still_in_play: self.in_play(*current_occupancy), // after reconcile
        });
      }
    }
  }
}

#[derive(serde::Serialize, Event)]
pub struct MaxBallsInPlayChanged {
  pub max_in_play: u8,
  pub delta: i8,
}

#[derive(serde::Serialize, Event)]
pub struct BallsDrained {
  pub still_in_play: u8,
  pub count: u8,
}
