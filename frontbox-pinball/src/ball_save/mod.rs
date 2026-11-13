mod ball_save_ext;
pub use ball_save_ext::*;

use std::collections::HashMap;

use frontbox_core::prelude::*;
use frontbox_led::prelude::*;

use crate::ball_management::{BallManager, Launch};
use crate::{BallDraining, TurnActive, TurnPhase, configs, metrics};

/// Saves balls that drain while a ball save is active, by relaunching them as though the drain never happened.
///
/// ## Per player
/// Ball save belongs in the player's system group (the `GameManager` player template), not at the root. It is only
/// active while that player's turn is beginning or active. When the turn ends it deactivates, which clears every
/// request, so nothing carries over to the next turn. Ball management, by contrast, is global.
///
/// ## Requests
/// Other systems don't turn ball save on and off; they place a request, much like declaring LEDs. Ball save is active
/// while any request is.
/// - Each system has at most one request. A new one replaces that system's previous one.
/// - Requests from different systems overlap: a timed request runs to its own deadline and never shortens another.
/// - A system's request is cleared if that system is despawned.
///
/// Requests are usually made through `BallSaveExt` on `SystemContext`, which passes the system id automatically:
///
/// ```rust,ignore
/// // for a while
/// ctx.request_ball_save(Duration::from_secs(12));
///
/// // or, for as long as the system wants
/// ctx.request_ball_save_until_released();
/// ctx.release_ball_save();
/// ```
///
/// A multiball usually wants the save timed from when its balls are in play rather than when they were asked for, as
/// launching them takes a while. Save until released, then time it once its eject completes:
///
/// ```rust,ignore
/// self.eject_id = ctx.eject_balls(2, Launch::Auto);
/// ctx.request_ball_save_until_released();
///
/// // in on_event
/// if let Some(done) = event.downcast_ref::<EjectCompleted>()
///   && Some(done.id) == self.eject_id
/// {
///   ctx.request_ball_save(Duration::from_secs(12));
/// }
/// ```
///
/// ## Turn start
/// Each turn starts with a request lasting the `BALL_SAVE_TIME` operator config (0 for off), made once the ball enters
/// play.
///
/// ## Saving
/// Ball save is a participant in the `BallDraining` routine, which the game manager runs for each drain. While active,
/// it saves every drain: it relaunches the balls, emits `BallSaved`, and halts the routine, so the drain doesn't stand
/// and later participants (an extra ball, say) don't take a step. Otherwise it completes its step and lets the drain
/// go on to them.
///
/// After the last request ends, a grace period keeps saving a little longer (a ball already heading for the outlane as
/// the save ends). The effect stops when the requests end, not after the grace period.
///
/// ## Setup
/// ```rust,ignore
/// // in the player template
/// BallSaveSystem::new()
///   .grace_period(Duration::from_secs(2))
///   .led_effect(ball_save_effect)
///
/// app.routine::<BallDraining>(routine![BallSaveSystem]);
/// ```
#[derive(Clone)]
pub struct BallSaveSystem {
  handle: SystemHandle,
  effect: Option<LedPlayback>,
  effect_playing: bool,
  /// Time remaining per requesting system, or `None` for until released
  requests: HashMap<u64, Option<Duration>>,
  grace_period: Duration,
  grace_remaining: Duration,
  /// Whether drains were being saved as of the last update, to log when that changes
  was_saving: bool,
}

impl BallSaveSystem {
  pub fn new() -> Self {
    Self {
      handle: SystemHandle::default(),
      grace_period: Duration::from_secs(2),
      effect: None,
      effect_playing: false,
      requests: HashMap::new(),
      grace_remaining: Duration::ZERO,
      was_saving: false,
    }
  }

  /// How long saving continues after the last request ends
  pub fn grace_period(mut self, grace_period: Duration) -> Self {
    self.grace_period = grace_period;
    self
  }

  /// An effect to play while there are requests
  pub fn led_effect(mut self, effect: LedPlayback) -> Self {
    self.effect = Some(effect.stopped());
    self
  }

  /// Save balls for `duration`, replacing any request already made by `system_id`
  pub fn request(&mut self, system_id: u64, duration: Duration, ctx: &ServiceContext) {
    self.set_request(system_id, Some(duration), &ctx.for_system(self.handle));
  }

  /// Save balls until `release` is called, replacing any request already made by `system_id`
  pub fn request_until_released(&mut self, system_id: u64, ctx: &ServiceContext) {
    self.set_request(system_id, None, &ctx.for_system(self.handle));
  }

  /// Remove the request made by `system_id`, if any. Other systems' requests are unaffected.
  pub fn release(&mut self, system_id: u64, ctx: &ServiceContext) {
    self.remove_request(system_id, &ctx.for_system(self.handle));
  }

  /// True while drains are being saved, including the grace period
  pub fn is_saving(&self) -> bool {
    !self.requests.is_empty() || self.grace_remaining > Duration::ZERO
  }

  fn set_request(&mut self, system_id: u64, remaining: Option<Duration>, ctx: &SystemContext) {
    self.requests.insert(system_id, remaining);
    self.grace_remaining = Duration::ZERO;
    self.update(ctx);
  }

  fn remove_request(&mut self, system_id: u64, ctx: &SystemContext) {
    if self.requests.remove(&system_id).is_some() && self.requests.is_empty() {
      self.grace_remaining = self.grace_period;
    }
    self.update(ctx);
  }

  fn clear(&mut self, ctx: &SystemContext) {
    self.requests.clear();
    self.grace_remaining = Duration::ZERO;
    self.update(ctx);
  }

  /// Bring the effect in line with the current requests
  fn update(&mut self, ctx: &SystemContext) {
    let requested = !self.requests.is_empty();
    if requested != self.effect_playing {
      self.effect_playing = requested;
      if let Some(effect) = self.effect.as_mut() {
        if requested {
          effect.reset();
          effect.play();
        } else {
          effect.stop(ctx);
        }
      }
    }

    let saving = self.is_saving();
    if saving != self.was_saving {
      self.was_saving = saving;
      if saving {
        log::info!(target: "frontbox::ball_save", "Ball save started");
      } else {
        log::info!(target: "frontbox::ball_save", "Ball save ended");
      }
    }
  }
}

impl System for BallSaveSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is_any(&[TurnPhase::Starting, TurnPhase::Active])
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    // Overriding skips the default LED deactivation; clearing stops the effect instead
    self.clear(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<TurnActive>() {
      let duration = ctx.config.get(&configs::BALL_SAVE_TIME);
      if !duration.is_zero() {
        self.set_request(self.handle.id(), Some(duration), ctx);
      }
    } else if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>() {
      self.remove_request(*id, ctx);
    }
  }

  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    let Some(BallDraining { count, .. }) = routine.downcast_ref::<BallDraining>() else {
      return;
    };
    if !self.is_saving() {
      ctx.routines.complete_step::<BallDraining>();
      return;
    }
    let Some(mut ball_manager) = ctx.systems.get::<BallManager>() else {
      log::error!(target: "frontbox::ball_save", "No BallManager to relaunch the saved ball(s); letting the drain stand");
      ctx.routines.complete_step::<BallDraining>();
      return;
    };

    log::info!(target: "frontbox::ball_save", "Saving {} ball(s)", count);
    ball_manager.eject_many(*count, Launch::Auto, ctx.into());
    ctx.metrics.add(&metrics::BALL_SAVES, *count as u64);
    ctx.emit(BallSaved { count: *count });
    ctx.routines.halt::<BallDraining>();
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    // A grace period started by a request expiring this tick starts counting next tick
    let in_grace = self.requests.is_empty() && self.grace_remaining > Duration::ZERO;

    let expired: Vec<u64> = self
      .requests
      .iter_mut()
      .filter_map(|(id, remaining)| {
        let remaining = remaining.as_mut()?;
        *remaining = remaining.saturating_sub(delta);
        (remaining.is_zero()).then_some(*id)
      })
      .collect();
    for id in expired {
      self.remove_request(id, ctx);
    }

    if in_grace {
      self.grace_remaining = self.grace_remaining.saturating_sub(delta);
      if self.grace_remaining.is_zero() {
        self.update(ctx);
      }
    }

    if self.effect_playing
      && let Some(effect) = self.effect.as_mut()
    {
      effect.apply(delta, ctx);
    }
  }
}

/// Emitted when a ball save relaunches drained balls
#[derive(serde::Serialize, Event)]
pub struct BallSaved {
  pub count: u8,
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use super::*;
  use crate::ball_management::BallManagement;

  const A: u64 = 1;
  const B: u64 = 2;

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
    fn cancel_ejects(&mut self, _ctx: &ServiceContext) {}
    fn ejects_pending(&self) -> bool {
      false
    }
    fn balls_in_play(&self, _ctx: &ServiceContext) -> u8 {
      0
    }
    fn is_ball_in_plunge_lane(&self, _ctx: &ServiceContext) -> bool {
      false
    }
    fn establish_ball_count(&mut self, _ctx: &ServiceContext) {}
    fn remove_missing_ball(&mut self, _ctx: &ServiceContext) {}
    fn balls_missing(&self, _ctx: &ServiceContext) -> u8 {
      0
    }
    fn set_balls_held(&mut self, _holder: u64, _count: u8, _ctx: &ServiceContext) {}
    fn balls_held(&self) -> u8 {
      0
    }
  }

  struct Fixture {
    save: BallSaveSystem,
    ctx: TestContext,
    ejects: Arc<Mutex<Vec<(u8, Launch)>>>,
  }

  impl Fixture {
    fn spawn() -> Self {
      Self::with(BallSaveSystem::new().grace_period(Duration::from_secs(2)))
    }

    fn with(mut save: BallSaveSystem) -> Self {
      let mut ctx = TestContext::default();
      let balls = FakeBallManagement::default();
      let ejects = balls.ejects.clone();
      ctx.insert_system(BallManager::new(balls));
      save.on_spawn(&ctx.sys_ctx());
      Self { save, ctx, ejects }
    }

    fn request(&mut self, id: u64, secs: u64) {
      self
        .save
        .request(id, Duration::from_secs(secs), &self.ctx.svc_ctx());
    }

    fn request_until_released(&mut self, id: u64) {
      self.save.request_until_released(id, &self.ctx.svc_ctx());
    }

    fn release(&mut self, id: u64) {
      self.save.release(id, &self.ctx.svc_ctx());
    }

    fn tick(&mut self, secs: u64) {
      self
        .save
        .on_tick(Duration::from_secs(secs), &self.ctx.sys_ctx());
    }

    /// Hand ball save the `BallDraining` step for a drain of `count` balls
    fn drain(&mut self, count: u8) {
      let draining = BallDraining {
        current_player: 0,
        turn: 0,
        count,
        in_play: 0,
      };
      self.save.on_routine(&draining, &self.ctx.sys_ctx());
    }

    fn saved_events(&mut self) -> Vec<u8> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<BallSaved>())
        .map(|e| e.count)
        .collect()
    }

    fn ejects(&self) -> Vec<(u8, Launch)> {
      self.ejects.lock().unwrap().clone()
    }
  }

  #[test]
  fn without_requests_drains_stand() {
    let mut f = Fixture::spawn();
    assert!(!f.save.is_saving());
    f.drain(1);
    assert!(f.ctx.routine_step_completed::<BallDraining>());
    assert!(f.ejects().is_empty());
  }

  #[test]
  fn while_requested_drains_are_relaunched_and_halt_the_routine() {
    let mut f = Fixture::spawn();
    f.request(A, 10);
    f.drain(2);
    assert!(f.ctx.routine_halted::<BallDraining>());
    assert_eq!(f.ejects(), vec![(2, Launch::Auto)]);
  }

  #[test]
  fn saved_drains_emit_ball_saved() {
    let mut f = Fixture::spawn();
    f.request(A, 10);
    f.drain(2);
    assert_eq!(f.saved_events(), vec![2]);
  }

  #[test]
  fn saved_balls_are_counted() {
    let mut f = Fixture::spawn();
    f.drain(1); // not saving
    f.request(A, 10);
    f.drain(2);
    let counts: Vec<_> = f
      .ctx
      .base
      .metrics
      .take_batch()
      .counts
      .iter()
      .map(|delta| (delta.metric, delta.count))
      .collect();
    assert_eq!(counts, vec![("ball_saves", 2)]);
  }

  #[test]
  fn timed_request_expires_then_grace_period_keeps_saving() {
    let mut f = Fixture::spawn();
    f.request(A, 5);
    f.tick(5);
    assert!(f.save.requests.is_empty());
    assert!(f.save.is_saving(), "grace period");

    f.tick(2);
    assert!(!f.save.is_saving());
  }

  #[test]
  fn overlapping_requests_run_to_the_later_deadline() {
    let mut f = Fixture::spawn();
    f.request(A, 5);
    f.request(B, 12);
    f.tick(6);
    assert!(f.save.is_saving());
    assert!(!f.save.requests.contains_key(&A));
    assert!(f.save.requests.contains_key(&B));
    assert!(f.save.grace_remaining.is_zero(), "no grace while B remains");
  }

  #[test]
  fn new_request_replaces_the_same_systems_previous_one() {
    let mut f = Fixture::spawn();
    f.request(A, 12);
    f.request(A, 2);
    f.tick(2);
    assert!(f.save.requests.is_empty(), "the 12s request was replaced");
  }

  #[test]
  fn request_until_released_lasts_until_released_then_grace() {
    let mut f = Fixture::spawn();
    f.request_until_released(A);
    f.tick(60);
    assert!(f.save.requests.contains_key(&A));

    f.release(A);
    assert!(f.save.is_saving(), "grace period");
    f.tick(2);
    assert!(!f.save.is_saving());
  }

  #[test]
  fn releasing_only_removes_that_systems_request() {
    let mut f = Fixture::spawn();
    f.request_until_released(A);
    f.release(B);
    assert!(f.save.requests.contains_key(&A));
  }

  #[test]
  fn despawned_systems_request_is_cleared() {
    let mut f = Fixture::spawn();
    f.request_until_released(A);
    f.request_until_released(B);
    f.save
      .on_event(&SystemDespawned::new(A, ROOT_GROUP), &f.ctx.sys_ctx());
    assert!(!f.save.requests.contains_key(&A));
    assert!(f.save.requests.contains_key(&B));
  }

  #[test]
  fn turn_start_requests_a_save() {
    let mut f = Fixture::spawn();
    f.save.on_event(&TurnActive::new(0, 0), &f.ctx.sys_ctx());
    assert!(f.save.is_saving());
    f.tick(8);
    assert!(f.save.requests.is_empty());
  }

  #[test]
  fn zero_ball_save_time_turn_start_requests_nothing() {
    let mut f = Fixture::spawn();
    f.ctx
      .svc_ctx()
      .config
      .set(&configs::BALL_SAVE_TIME, Duration::ZERO);
    f.save.on_event(&TurnActive::new(0, 0), &f.ctx.sys_ctx());
    assert!(!f.save.is_saving());
  }

  #[test]
  fn deactivating_clears_everything_without_grace() {
    let mut f = Fixture::spawn();
    f.request(A, 10);
    f.request_until_released(B);
    f.save.on_deactivate(&f.ctx.sys_ctx());
    assert!(!f.save.is_saving());
    f.drain(1);
    assert!(f.ctx.routine_step_completed::<BallDraining>());
  }
}
