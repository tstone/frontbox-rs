use frontbox::prelude::*;

use crate::GameStarted;
use crate::ball_management::ball_management_config::BallManagementConfig;
use crate::ball_management::trough::TroughSystem;
use crate::ball_management::{BallsDrained, MaxBallsInPlayChanged, TroughOccupancyChanged};

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
      max_balls_in_play: 0, // set on spawn
      config,
      locked_balls: 0,
    }
  }

  pub fn eject(&mut self, ctx: &ServiceContext) {
    ctx.expect::<TroughSystem>(self.handle).eject(ctx);
  }

  pub fn eject_many(&mut self, count: u8, ctx: &ServiceContext) {
    ctx
      .expect::<TroughSystem>(self.handle)
      .eject_many(count, ctx);
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

    if self.locked_balls > prior {
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

  /// If somehow the trough reports more balls than we were expecting
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
      log::warn!(target: "frontbox::ball_management", "{} unexpected ball(s) entered the trough, increasing max balls in play", rem);
      self.max_balls_in_play += rem;
      ctx.emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: rem as i8,
      });
    }
  }
}

impl System for BallManagementSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
    let trough = TroughSystem::new(self.config.clone());
    // Until a game starts and establishes it, assume the balls in the trough now are all the balls there are
    self.max_balls_in_play = trough.read_switch_occupancy(ctx.into());
    ctx.spawn_system(trough);
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

#[cfg(test)]
mod tests {
  use super::*;

  const TROUGH: [&str; 5] = ["t1", "t2", "t3", "t4", "t5"];

  struct Fixture {
    system: BallManagementSystem,
    ctx: TestContext,
  }

  impl Fixture {
    /// Spawns ball management with `balls` in a 5 switch trough, alongside a spawned trough for it to query
    fn spawn(balls: usize) -> Self {
      let mut ctx = TestContext::default();
      for (id, name) in TROUGH.iter().enumerate() {
        ctx.insert_switch(Switch {
          name,
          id,
          ..Default::default()
        });
      }
      ctx.update_switch_states((0..TROUGH.len()).map(|i| state(i < balls)).collect());

      let config = BallManagementConfig {
        trough_switch_names: TROUGH.to_vec(),
        eject_coil_name: "eject",
        ..Default::default()
      };

      let mut trough = TroughSystem::new(config.clone());
      trough.on_spawn(&ctx.sys_ctx());
      ctx.insert_system(trough);

      let mut system = BallManagementSystem::new(config);
      system.on_spawn(&ctx.sys_ctx());

      let mut fixture = Self { system, ctx };
      fixture.ctx.events_emitted(); // discard anything from setup
      fixture
    }

    fn deliver(&mut self, event: &dyn Event) {
      self.system.on_event(event, &self.ctx.sys_ctx());
    }

    /// Simulate the trough reporting a settled occupancy change
    fn trough_changed(&mut self, current_occupancy: u8, entered: u8, exited: u8) {
      self.deliver(&TroughOccupancyChanged {
        current_occupancy,
        entered,
        exited,
      });
    }

    fn lock(&mut self) {
      self
        .system
        .ball_temporarily_removed_from_play(&self.ctx.svc_ctx());
    }

    fn unlock(&mut self) {
      self.system.ball_returned_to_play(&self.ctx.svc_ctx());
    }

    fn balls_in_play(&self) -> u8 {
      self.system.balls_in_play(&self.ctx.svc_ctx())
    }

    /// Short names of all events emitted since the last call
    fn events(&mut self) -> Vec<String> {
      self
        .ctx
        .events_emitted()
        .iter()
        .map(|e| e.short_name())
        .collect()
    }

    /// (count, still_in_play) of all drain events emitted since the last call
    fn drained_events(&mut self) -> Vec<(u8, u8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<BallsDrained>())
        .map(|e| (e.count, e.still_in_play))
        .collect()
    }

    /// (max_in_play, delta) of all max balls in play events emitted since the last call
    fn max_changed_events(&mut self) -> Vec<(u8, i8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<MaxBallsInPlayChanged>())
        .map(|e| (e.max_in_play, e.delta))
        .collect()
    }
  }

  fn state(closed: bool) -> SwitchState {
    if closed {
      SwitchState::Closed
    } else {
      SwitchState::Open
    }
  }

  // -- Spawn --

  #[test]
  fn spawn_establishes_max_from_balls_present() {
    let f = Fixture::spawn(4);
    assert_eq!(f.system.max_balls_in_play, 4);
    assert_eq!(
      f.balls_in_play(),
      0,
      "an empty trough switch is not a ball in play"
    );
  }

  // -- Drains --

  #[test]
  fn exit_only_change_emits_nothing() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    assert!(f.events().is_empty());
  }

  #[test]
  fn last_ball_draining_reports_none_still_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.trough_changed(5, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  #[test]
  fn drain_during_multiball_reports_balls_still_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(2, 0, 3);
    f.trough_changed(3, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 2)]);
  }

  #[test]
  fn two_balls_draining_together_report_a_single_event() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(2, 0, 3);
    f.trough_changed(4, 2, 0);
    assert_eq!(f.drained_events(), vec![(2, 1)]);
  }

  #[test]
  fn eject_and_drain_in_same_report_counts_the_drain() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.trough_changed(4, 1, 1);
    assert_eq!(f.drained_events(), vec![(1, 1)]);
  }

  // -- Locked balls --

  #[test]
  fn lock_emits_max_changed() {
    let mut f = Fixture::spawn(5);
    f.lock();
    assert_eq!(f.max_changed_events(), vec![(4, -1)]);
  }

  #[test]
  fn unlock_emits_max_changed() {
    let mut f = Fixture::spawn(5);
    f.lock();
    f.events();
    f.unlock();
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
  }

  #[test]
  fn unlock_with_nothing_locked_emits_nothing() {
    let mut f = Fixture::spawn(5);
    f.unlock();
    assert!(f.events().is_empty());
    assert_eq!(f.system.locked_balls, 0);
  }

  #[test]
  fn locked_ball_is_not_counted_as_still_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock(); // ball in play goes into the lock
    f.trough_changed(3, 0, 1); // replacement ball ejected
    f.trough_changed(4, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  // -- Unexpected balls --

  #[test]
  fn unexpected_ball_with_ball_locked_is_treated_as_released() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock();
    f.trough_changed(3, 0, 1);
    f.trough_changed(4, 1, 0); // last ball in play drains
    f.events();

    // the locked ball comes loose without being returned to play, and drains
    f.trough_changed(5, 1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.system.locked_balls, 0);
    assert_eq!(f.system.max_balls_in_play, 5);
  }

  #[test]
  fn unexpected_ball_with_nothing_locked_raises_max() {
    let mut f = Fixture::spawn(4);
    // a ball that was stuck on the playfield comes home
    f.trough_changed(5, 1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.system.max_balls_in_play, 5);
  }

  #[test]
  fn unexpected_ball_alongside_drain_reports_only_the_drain() {
    let mut f = Fixture::spawn(4);
    f.trough_changed(3, 0, 1);
    f.events();

    // the ball in play drains along with a previously stuck ball
    f.trough_changed(5, 2, 0);
    let events = f.ctx.events_emitted();
    let drained: Vec<(u8, u8)> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<BallsDrained>())
      .map(|e| (e.count, e.still_in_play))
      .collect();
    let max_changed: Vec<(u8, i8)> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<MaxBallsInPlayChanged>())
      .map(|e| (e.max_in_play, e.delta))
      .collect();
    assert_eq!(drained, vec![(1, 0)]);
    assert_eq!(max_changed, vec![(5, 1)]);
  }

  // -- Game start --

  #[test]
  fn game_started_reestablishes_max_from_trough_and_clears_locks() {
    let mut f = Fixture::spawn(5);
    f.lock();
    f.system.max_balls_in_play = 6; // drifted during the previous game
    f.events();

    f.deliver(&GameStarted);
    assert_eq!(f.system.max_balls_in_play, 5);
    assert_eq!(f.system.locked_balls, 0);
    assert_eq!(f.balls_in_play(), 0);
    assert!(f.events().is_empty());
  }

  // -- Known limitations --

  #[test]
  fn locked_ball_draining_unannounced_during_multiball_undercounts_still_in_play() {
    // Known limitation: without lock switches, a locked ball that comes loose looks like an ordinary drain until the
    // trough holds more balls than expected
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock();
    f.trough_changed(2, 0, 2); // 2 balls in play
    f.trough_changed(3, 1, 0); // the locked ball comes loose and drains
    assert_eq!(f.drained_events(), vec![(1, 1)], "2 are actually still in play");
  }
}
