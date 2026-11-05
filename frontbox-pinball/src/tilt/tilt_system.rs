use frontbox_core::prelude::*;

use crate::{GameManagementExt, TiltWarned, TurnPhase, TurnStarting, configs, metrics};

/// Reads the tilt bob: each swing warns the player, and once they're out of warnings, the next one tilts them.
///
/// ## Per player
/// Belongs in the player template, like `BallSaveSystem`, as the warnings are the player's. It's only active while
/// their turn is starting or active, so it stops reading the bob once the turn is tilted or over. Handling the tilt
/// itself (ending the turn, ignoring points) is the game manager's: see `GameManagement::tilt`.
///
/// ## Warnings
/// The player has `TILT_WARNINGS` warnings (operator config). Each swing uses one and emits `TiltWarned`; a swing once
/// none are left tilts. With `TILT_WARNINGS_PER_TURN` on, they're given back at the start of every turn, an extra
/// ball's included; otherwise they last the game. Another can be earned with `grant_warning` (a mystery award, say),
/// which lasts until warnings are next given back.
///
/// The bob keeps swinging for a moment after it's hit, so it's ignored for a settle time after each warning. It's also
/// ignored until the turn's ball is served, and for a grace period after.
///
/// ## Setup
/// ```rust,ignore
/// // in the player template
/// TiltSystem::new(&switches::TILT_BOB)
///   .settle_time(Duration::from_millis(1500))
/// ```
#[derive(Clone)]
pub struct TiltSystem {
  bob: SwitchQ,
  settle_time: Duration,
  grace_period: Duration,
  /// Warnings left before the next swing tilts
  warnings: u8,
  /// Whether the turn's ball has been served, so the bob counts
  served: bool,
  /// Time left ignoring the bob, after a warning or the serve
  ignoring: Duration,
}

impl TiltSystem {
  pub fn new(bob: impl Into<SwitchQ>) -> Self {
    Self {
      bob: bob.into(),
      settle_time: Duration::from_secs(1),
      grace_period: Duration::from_secs(2),
      warnings: 0,
      served: false,
      ignoring: Duration::ZERO,
    }
  }

  /// How long the bob is ignored after each warning, while it settles
  pub fn settle_time(mut self, settle_time: Duration) -> Self {
    self.settle_time = settle_time;
    self
  }

  /// How long the bob is ignored after the turn's ball is served
  pub fn grace_period(mut self, grace_period: Duration) -> Self {
    self.grace_period = grace_period;
    self
  }

  /// Warnings left before the next swing tilts
  pub fn warnings(&self) -> u8 {
    self.warnings
  }

  /// Give the player another warning, until warnings are next given back
  pub fn grant_warning(&mut self, _ctx: &ServiceContext) {
    self.warnings = self.warnings.saturating_add(1);
    log::info!(target: "frontbox_core::tilt", "Tilt warning granted, {} left", self.warnings);
  }

  fn bob_swung(&mut self, ctx: &SystemContext) {
    if !self.served || !self.ignoring.is_zero() {
      return;
    }
    if self.warnings == 0 {
      log::info!(target: "frontbox_core::tilt", "Tilted");
      ctx.tilt();
      return;
    }

    self.warnings -= 1;
    self.ignoring = self.settle_time;
    log::info!(target: "frontbox_core::tilt", "Tilt warning, {} left", self.warnings);
    ctx.metrics.inc(&metrics::TILT_WARNINGS);
    ctx.emit(TiltWarned {
      remaining: self.warnings,
    });
  }
}

impl System for TiltSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // the player's group is spawned as they're added, so this is the start of their game
    self.warnings = ctx.config.get(&configs::TILT_WARNINGS);
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is_any(&[TurnPhase::Starting, TurnPhase::Active])
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<RoutineStarted<TurnStarting>>() {
      self.served = false;
      if ctx.config.get(&configs::TILT_WARNINGS_PER_TURN) {
        self.warnings = ctx.config.get(&configs::TILT_WARNINGS);
      }
    } else if event.is::<RoutineCompleted<TurnStarting>>() {
      self.served = true;
      self.ignoring = self.grace_period;
    } else if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>()
      && self.bob.matches(switch)
    {
      self.bob_swung(ctx);
    }
  }

  fn on_tick(&mut self, delta: Duration, _ctx: &SystemContext) {
    self.ignoring = self.ignoring.saturating_sub(delta);
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU8, Ordering};

  use super::*;
  use crate::{GameEndReason, GameManagement, GameManager, GameState, TurnStartReason};

  const BOB: &str = "tilt_bob";
  const SETTLE: Duration = Duration::from_secs(1);
  const GRACE: Duration = Duration::from_secs(2);

  struct Fixture {
    system: TiltSystem,
    ctx: TestContext,
    /// How many times the game manager was told to tilt
    tilts: Arc<AtomicU8>,
  }

  impl Fixture {
    /// A player with `warnings` warnings, with the ball served and the grace period over
    fn playing(warnings: u8) -> Self {
      let mut ctx = TestContext::default();
      ctx.svc_ctx().config.set(&configs::TILT_WARNINGS, warnings);
      let tilts = Arc::new(AtomicU8::new(0));
      ctx.insert_system(GameManager::new(FakeGame {
        tilts: tilts.clone(),
      }));
      let mut system = TiltSystem::new(SwitchQ::name(BOB))
        .settle_time(SETTLE)
        .grace_period(GRACE);
      system.on_spawn(&ctx.sys_ctx());
      let mut f = Self { system, ctx, tilts };
      f.start_turn(TurnStartReason::Ordered);
      f.serve();
      f.tick(GRACE);
      f
    }

    fn start_turn(&mut self, reason: TurnStartReason) {
      self.event(RoutineStarted {
        routine: turn_starting(reason),
      });
    }

    fn serve(&mut self) {
      self.event(RoutineCompleted {
        routine: turn_starting(TurnStartReason::Ordered),
        completion: RoutineCompletion::AllSteps,
      });
    }

    fn swing(&mut self) {
      self.event(SwitchClosed::new(Switch {
        name: BOB,
        ..Default::default()
      }));
    }

    fn tick(&mut self, delta: Duration) {
      self.system.on_tick(delta, &self.ctx.sys_ctx());
    }

    fn event(&mut self, event: impl Event) {
      self.system.on_event(&event, &self.ctx.sys_ctx());
    }

    fn tilts(&self) -> u8 {
      self.tilts.load(Ordering::SeqCst)
    }

    /// `remaining` of each `TiltWarned` emitted since the last check
    fn warned(&mut self) -> Vec<u8> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<TiltWarned>())
        .map(|e| e.remaining)
        .collect()
    }
  }

  fn turn_starting(reason: TurnStartReason) -> TurnStarting {
    TurnStarting {
      current_player: 0,
      turn: 0,
      reason,
    }
  }

  /// A game manager that only counts tilts
  struct FakeGame {
    tilts: Arc<AtomicU8>,
  }

  impl System for FakeGame {}

  impl GameManagement for FakeGame {
    fn add_player(&mut self, _ctx: &ServiceContext) {}
    fn quit_game(&mut self, _reason: GameEndReason, _ctx: &ServiceContext) {}
    fn tilt(&mut self, _ctx: &ServiceContext) {
      self.tilts.fetch_add(1, Ordering::SeqCst);
    }
    fn game_state(&self) -> Option<&GameState> {
      None
    }
    fn add_points(&mut self, _points: u32, _ctx: &ServiceContext) {}
    fn set_multiplier(&mut self, _multiplier: f32) {}
    fn clear_multiplier(&mut self) {}
    fn grant_extra_ball(&mut self, _ctx: &ServiceContext) -> bool {
      false
    }
  }

  #[test]
  fn each_swing_warns_until_none_are_left_then_tilts() {
    let mut f = Fixture::playing(2);
    f.swing();
    f.tick(SETTLE);
    f.swing();
    f.tick(SETTLE);
    assert_eq!(f.warned(), vec![1, 0]);
    assert_eq!(f.tilts(), 0);

    f.swing();
    assert_eq!(f.tilts(), 1);
    assert!(f.warned().is_empty());
  }

  #[test]
  fn with_no_warnings_the_first_swing_tilts() {
    let mut f = Fixture::playing(0);
    f.swing();
    assert_eq!(f.tilts(), 1);
  }

  #[test]
  fn the_bob_is_ignored_while_it_settles() {
    let mut f = Fixture::playing(2);
    f.swing();
    f.tick(SETTLE / 2);
    f.swing();
    assert_eq!(
      f.warned(),
      vec![1],
      "the second swing is the same hit settling"
    );
  }

  #[test]
  fn the_bob_is_ignored_until_the_ball_is_served_and_for_a_grace_period() {
    let mut f = Fixture::playing(2);
    f.start_turn(TurnStartReason::Ordered);
    f.swing();
    f.serve();
    f.tick(GRACE / 2);
    f.swing();
    assert!(f.warned().is_empty());

    f.tick(GRACE);
    f.swing();
    assert_eq!(f.warned(), vec![1]);
  }

  #[test]
  fn warnings_are_given_back_each_turn_including_an_extra_ball() {
    let mut f = Fixture::playing(2);
    f.swing();
    f.tick(SETTLE);
    f.swing();
    assert_eq!(f.system.warnings(), 0);

    f.start_turn(TurnStartReason::ExtraBall);
    assert_eq!(f.system.warnings(), 2);
  }

  #[test]
  fn warnings_last_the_game_unless_given_back_per_turn() {
    let mut f = Fixture::playing(2);
    f.ctx
      .svc_ctx()
      .config
      .set(&configs::TILT_WARNINGS_PER_TURN, false);
    f.swing();
    f.start_turn(TurnStartReason::Ordered);
    assert_eq!(f.system.warnings(), 1);
  }

  #[test]
  fn an_earned_warning_lasts_until_warnings_are_given_back() {
    let mut f = Fixture::playing(1);
    f.system.grant_warning(&f.ctx.svc_ctx());
    f.swing();
    f.tick(SETTLE);
    f.swing();
    f.tick(SETTLE);
    assert_eq!(f.warned(), vec![1, 0]);
    assert_eq!(f.tilts(), 0);

    f.system.grant_warning(&f.ctx.svc_ctx());
    f.start_turn(TurnStartReason::Ordered);
    assert_eq!(f.system.warnings(), 1, "back to the operator's setting");
  }
}
