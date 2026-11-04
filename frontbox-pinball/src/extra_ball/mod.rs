mod extra_ball_ext;
pub use extra_ball_ext::*;

use frontbox::prelude::*;

use crate::ball_management::{BallManager, Launch};
use crate::{BallDraining, TurnPhase, metrics};

/// Extends a player's turn when their last ball drains, by serving them a new one, once for each extra ball granted.
///
/// ## Per player
/// Extra balls belong in the player's system group (the `GameManager` player template), not at the root, so each player
/// keeps their own. Unused extra balls carry over from turn to turn until the game ends.
///
/// ## Granting
/// Other systems grant extra balls, usually through `ExtraBallExt` on `SystemContext`:
///
/// ```rust,ignore
/// ctx.grant_extra_ball();
/// ```
///
/// ## Saving
/// Extra ball is a participant in the `BallDraining` routine, which the game manager runs for each drain. When the last
/// ball in play drains and the player has an extra ball, it uses one: it serves a new ball to the plunger, emits
/// `ExtraBallUsed`, and halts the routine, so the turn goes on. Drains that leave other balls in play, or with no extra
/// balls left, complete its step and stand.
///
/// List it after ball save, so a drain that ball save would have saved doesn't use up an extra ball:
///
/// ```rust,ignore
/// // in the player template
/// ExtraBallSystem::new()
///
/// app.routine::<BallDraining>(routine![BallSaveSystem, ExtraBallSystem]);
/// ```
#[derive(Clone)]
pub struct ExtraBallSystem {
  handle: SystemHandle,
  /// Extra balls granted and not yet used
  count: u8,
}

impl ExtraBallSystem {
  pub fn new() -> Self {
    Self {
      handle: SystemHandle::default(),
      count: 0,
    }
  }

  /// Give the player an extra ball, used the next time their last ball drains
  pub fn grant(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    self.count = self.count.saturating_add(1);
    log::info!(target: "frontbox::extraball", "Extra ball granted, {} unused", self.count);
    ctx.emit(ExtraBallGranted { count: self.count });
  }

  /// Extra balls granted and not yet used
  pub fn count(&self) -> u8 {
    self.count
  }
}

impl System for ExtraBallSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.in_any_state(&[TurnPhase::Starting, TurnPhase::Active])
  }

  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    let Some(BallDraining { in_play, .. }) = routine.downcast_ref::<BallDraining>() else {
      return;
    };
    if *in_play > 0 || self.count == 0 {
      ctx.complete_routine_step::<BallDraining>();
      return;
    }
    let Some(mut ball_manager) = ctx.get::<BallManager>() else {
      log::error!(target: "frontbox::extraball", "No BallManager to serve the extra ball; letting the drain stand");
      ctx.complete_routine_step::<BallDraining>();
      return;
    };

    self.count -= 1;
    log::info!(target: "frontbox::extraball", "Using an extra ball, {} unused", self.count);
    ball_manager.eject(Launch::Player, ctx.into());
    ctx.metrics.add(&metrics::EXTRA_BALLS, 1);
    ctx.emit(ExtraBallUsed {
      remaining: self.count,
    });
    ctx.halt_routine::<BallDraining>();
  }
}

/// Emitted when a player is granted an extra ball
#[derive(serde::Serialize, Event)]
pub struct ExtraBallGranted {
  /// Extra balls the player now has unused, including this one
  pub count: u8,
}

/// Emitted when an extra ball is used: the player's last ball drained, and a new one is being served
#[derive(serde::Serialize, Event)]
pub struct ExtraBallUsed {
  /// Extra balls the player still has unused
  pub remaining: u8,
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use super::*;
  use crate::ball_management::BallManagement;

  /// Records ejects, standing in for real ball management
  #[derive(Clone, Default)]
  struct FakeBallManagement {
    ejects: Arc<Mutex<Vec<(u8, Launch)>>>,
  }

  impl System for FakeBallManagement {}

  impl BallManagement for FakeBallManagement {
    fn eject(&mut self, launch: Launch, ctx: &ServiceContext) -> u64 {
      self.eject_many(1, launch, ctx)
    }
    fn eject_many(&mut self, count: u8, launch: Launch, _ctx: &ServiceContext) -> u64 {
      let mut ejects = self.ejects.lock().unwrap();
      ejects.push((count, launch));
      ejects.len() as u64
    }
    fn can_auto_launch(&self) -> bool {
      true
    }
    fn balls_in_play(&self, _ctx: &ServiceContext) -> u8 {
      0
    }
    fn establish_ball_count(&mut self, _ctx: &ServiceContext) {}
    fn ball_temporarily_removed_from_play(&mut self, _ctx: &ServiceContext) {}
    fn ball_returned_to_play(&mut self, _ctx: &ServiceContext) {}
  }

  struct Fixture {
    extra_ball: ExtraBallSystem,
    ctx: TestContext,
    ejects: Arc<Mutex<Vec<(u8, Launch)>>>,
  }

  impl Fixture {
    fn spawn() -> Self {
      let mut ctx = TestContext::default();
      let balls = FakeBallManagement::default();
      let ejects = balls.ejects.clone();
      ctx.insert_system(BallManager::new(balls));
      let mut extra_ball = ExtraBallSystem::new();
      extra_ball.on_spawn(&ctx.sys_ctx());
      Self {
        extra_ball,
        ctx,
        ejects,
      }
    }

    fn grant(&mut self) {
      self.extra_ball.grant(&self.ctx.svc_ctx());
    }

    /// Hand extra ball the `BallDraining` step for a drain that leaves `in_play` balls
    fn drain(&mut self, in_play: u8) {
      let draining = BallDraining {
        current_player: 0,
        turn: 0,
        count: 1,
        in_play,
      };
      self.extra_ball.on_routine(&draining, &self.ctx.sys_ctx());
    }

    fn ejects(&self) -> Vec<(u8, Launch)> {
      self.ejects.lock().unwrap().clone()
    }
  }

  #[test]
  fn without_extra_balls_drains_stand() {
    let mut f = Fixture::spawn();
    f.drain(0);
    assert!(f.ctx.routine_step_completed::<BallDraining>());
    assert!(f.ejects().is_empty());
  }

  #[test]
  fn last_ball_drain_uses_an_extra_ball_and_halts_the_routine() {
    let mut f = Fixture::spawn();
    f.grant();
    f.drain(0);
    assert!(f.ctx.routine_halted::<BallDraining>());
    assert_eq!(f.ejects(), vec![(1, Launch::Player)]);
    assert_eq!(f.extra_ball.count(), 0);
  }

  #[test]
  fn drains_with_balls_still_in_play_stand_and_keep_the_extra_ball() {
    let mut f = Fixture::spawn();
    f.grant();
    f.drain(1);
    assert!(f.ctx.routine_step_completed::<BallDraining>());
    assert!(f.ejects().is_empty());
    assert_eq!(f.extra_ball.count(), 1);
  }

  #[test]
  fn each_extra_ball_is_used_once() {
    let mut f = Fixture::spawn();
    f.grant();
    f.grant();
    f.drain(0);
    f.drain(0);
    assert_eq!(f.ejects().len(), 2);
    f.drain(0);
    assert!(f.ctx.routine_step_completed::<BallDraining>());
    assert_eq!(f.ejects().len(), 2);
  }

  #[test]
  fn granting_and_using_emit_events() {
    let mut f = Fixture::spawn();
    f.grant();
    f.grant();
    f.drain(0);
    let events = f.ctx.events_emitted();
    let granted: Vec<_> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<ExtraBallGranted>())
      .map(|e| e.count)
      .collect();
    let used: Vec<_> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<ExtraBallUsed>())
      .map(|e| e.remaining)
      .collect();
    assert_eq!(granted, vec![1, 2]);
    assert_eq!(used, vec![1]);
  }

  #[test]
  fn used_extra_balls_are_counted() {
    let mut f = Fixture::spawn();
    f.grant();
    f.drain(0);
    let counts: Vec<_> = f
      .ctx
      .base
      .metrics
      .take_batch()
      .counts
      .iter()
      .map(|delta| (delta.metric, delta.count))
      .collect();
    assert_eq!(counts, vec![("extra_balls", 1)]);
  }
}
