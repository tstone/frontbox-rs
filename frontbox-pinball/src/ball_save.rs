use std::collections::HashMap;

use frontbox::prelude::*;

use crate::ball_management::{BallManager, BallsExitedPlay, Launch};
use crate::{GameManager, PlayerTurnActive, TurnState};

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
/// // e.g. a mode starting a multiball
/// ctx.eject_balls(2, Launch::Auto);
/// ctx.request_ball_save(Duration::from_secs(12));
///
/// // or, for as long as the mode wants
/// ctx.request_ball_save_until_released();
/// ctx.release_ball_save();
/// ```
///
/// ## Saving
/// While active, every drain is saved: the `BallsExitedPlay` event is halted, since a saved drain is treated as not
/// having happened, and the balls are relaunched. `BallSaved` is emitted for anything that needs to know.
///
/// After the last request ends, a grace period keeps saving a little longer (a ball already heading for the outlane as
/// the save ends). The effect stops when the requests end, not after the grace period.
///
/// ## Setup
/// ```rust,ignore
/// BallSaveSystem::new()
///   .on_turn_start(Duration::from_secs(8))
///   .grace_period(Duration::from_secs(2))
///   .effect(ball_save_effect)
/// ```
#[derive(Clone)]
pub struct BallSaveSystem {
  handle: SystemHandle,
  on_turn_start: Option<Duration>,
  grace_period: Duration,
  effect: Option<LedProgram1d>,
  effect_playing: bool,
  /// Time remaining per requesting system, or `None` for until released
  requests: HashMap<u64, Option<Duration>>,
  grace_remaining: Duration,
  interrupt_registered: bool,
}

impl BallSaveSystem {
  pub fn interrupt_priority() -> u16 {
    25
  }

  pub fn new() -> Self {
    Self {
      handle: SystemHandle::default(),
      on_turn_start: None,
      grace_period: Duration::from_secs(2),
      effect: None,
      effect_playing: false,
      requests: HashMap::new(),
      grace_remaining: Duration::ZERO,
      interrupt_registered: false,
    }
  }

  /// Request a ball save for this long once the ball enters play at the start of each turn
  pub fn on_turn_start(mut self, duration: Duration) -> Self {
    self.on_turn_start = Some(duration);
    self
  }

  /// How long saving continues after the last request ends
  pub fn grace_period(mut self, grace_period: Duration) -> Self {
    self.grace_period = grace_period;
    self
  }

  /// An effect to play while there are requests
  pub fn effect(mut self, effect: LedProgram1d) -> Self {
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

  /// Bring the effect and the drain interrupt in line with the current requests
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
    if saving != self.interrupt_registered {
      self.interrupt_registered = saving;
      if saving {
        log::info!(target: "frontbox::ballsave", "Ball save started");
        ctx.register_interrupt::<BallsExitedPlay>(Self::interrupt_priority());
      } else {
        log::info!(target: "frontbox::ballsave", "Ball save ended");
        ctx.unregister_interrupt::<BallsExitedPlay>();
      }
    }
  }
}

impl System for BallSaveSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx
      .get::<GameManager>()
      .and_then(|manager| manager.turn_state().cloned())
      .map(|state| state == TurnState::Beginning || state == TurnState::Active)
      .unwrap_or(false)
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    // Overriding skips the default LED deactivation; clearing stops the effect instead
    self.clear(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<PlayerTurnActive>()
      && let Some(duration) = self.on_turn_start
    {
      self.set_request(self.handle.id, Some(duration), ctx);
    } else if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>() {
      self.remove_request(*id, ctx);
    }
  }

  fn on_interrupt(&mut self, event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
    let Some(BallsExitedPlay { count, .. }) = event.downcast_ref::<BallsExitedPlay>() else {
      return InterruptResult::Continue;
    };
    if !self.is_saving() {
      return InterruptResult::Continue;
    }
    let Some(mut ball_manager) = ctx.get::<BallManager>() else {
      log::error!(target: "frontbox::ballsave", "No BallManager to relaunch the saved ball(s); letting the drain through");
      return InterruptResult::Continue;
    };

    log::info!(target: "frontbox::ballsave", "Saving {} ball(s)", count);
    ball_manager.eject_many(*count, Launch::Auto, ctx.into());
    ctx.emit(BallSaved { count: *count });
    InterruptResult::Halt
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

/// Emitted when a ball save relaunches drained balls (the drain itself is halted)
#[derive(serde::Serialize, Event)]
pub struct BallSaved {
  pub count: u8,
}

pub trait BallSaveExt {
  /// Save balls for `duration`, replacing any request this system already made
  fn request_ball_save(&self, duration: Duration);
  /// Save balls until `release_ball_save`, replacing any request this system already made
  fn request_ball_save_until_released(&self);
  /// Remove this system's request, if any
  fn release_ball_save(&self);
  /// True while drains are being saved, including the grace period
  fn is_ball_save_active(&self) -> bool;
}

impl<'a> BallSaveExt for SystemContext<'a> {
  fn request_ball_save(&self, duration: Duration) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| {
      ball_save.request(id, duration, self.into())
    });
  }

  fn request_ball_save_until_released(&self) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| {
      ball_save.request_until_released(id, self.into())
    });
  }

  fn release_ball_save(&self) {
    let id = self.current_system_id();
    with_ball_save(self, |ball_save| ball_save.release(id, self.into()));
  }

  fn is_ball_save_active(&self) -> bool {
    self
      .get::<BallSaveSystem>()
      .map(|ball_save| ball_save.is_saving())
      .unwrap_or(false)
  }
}

fn with_ball_save(ctx: &SystemContext, f: impl FnOnce(&mut BallSaveSystem)) {
  if let Some(mut system) = ctx.get::<BallSaveSystem>() {
    f(&mut system);
  } else {
    log::warn!(target: "frontbox::ballsave", "BallSaveSystem not running.");
  }
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
    fn eject(&mut self, launch: Launch, _ctx: &ServiceContext) {
      self.ejects.lock().unwrap().push((1, launch));
    }
    fn eject_many(&mut self, count: u8, launch: Launch, _ctx: &ServiceContext) {
      self.ejects.lock().unwrap().push((count, launch));
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

    fn drain(&mut self, count: u8) -> InterruptResult {
      self
        .save
        .on_interrupt(&BallsExitedPlay { count, in_play: 0 }, &self.ctx.sys_ctx())
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
  fn without_requests_drains_go_through() {
    let mut f = Fixture::spawn();
    assert!(!f.save.is_saving());
    assert_eq!(f.drain(1), InterruptResult::Continue);
    assert!(f.ejects().is_empty());
  }

  #[test]
  fn while_requested_drains_are_halted_and_relaunched() {
    let mut f = Fixture::spawn();
    f.request(A, 10);
    assert_eq!(f.drain(2), InterruptResult::Halt);
    assert_eq!(f.ejects(), vec![(2, Launch::Auto)]);
    assert_eq!(f.saved_events(), vec![2]);
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
    let mut f = Fixture::with(BallSaveSystem::new().on_turn_start(Duration::from_secs(8)));
    f.save
      .on_event(&PlayerTurnActive::new(0, 0), &f.ctx.sys_ctx());
    assert!(f.save.is_saving());
    f.tick(8);
    assert!(f.save.requests.is_empty());
  }

  #[test]
  fn without_turn_start_option_turn_start_requests_nothing() {
    let mut f = Fixture::spawn();
    f.save
      .on_event(&PlayerTurnActive::new(0, 0), &f.ctx.sys_ctx());
    assert!(!f.save.is_saving());
  }

  #[test]
  fn deactivating_clears_everything_without_grace() {
    let mut f = Fixture::spawn();
    f.request(A, 10);
    f.request_until_released(B);
    f.save.on_deactivate(&f.ctx.sys_ctx());
    assert!(!f.save.is_saving());
    assert_eq!(f.drain(1), InterruptResult::Continue);
  }
}
