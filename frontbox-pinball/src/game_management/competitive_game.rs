use std::collections::VecDeque;
use std::time::Instant;

use frontbox::prelude::*;

use crate::ball_management::{BallEnteredPlay, BallManager, BallsExitedPlay, Launch};
use crate::*;

static PLAYER_GROUP_NAMES: [&str; 6] = [
  "player1", "player2", "player3", "player4", "player5", "player6",
];

/// CompetitiveGame manages the game state against players competing against each other
///
///   1. Systems are organized by player, so that each player can have their own set of systems that are active only during their turn.
///   2. Player turn management
///   3. Start game management
///
/// ## Outputs
/// When the first player is added, the game starts:
/// - Routine: `GameStarting` - Started at the root. The first turn begins on its `RoutineCompleted`, and players can
///   still be added while it runs.
///
/// Each turn goes through these, in order. The routines are started in the current player's group, so their
/// participants can be in the player template as well as at the root.
/// - Routine: `TurnStarting` - Started at the start of a player's turn, once their group is active. The ball is ejected
///   on its `RoutineCompleted`, so participants can hold the serve (e.g. a player-up display).
/// - Event: `TurnActive` - Emitted once per turn, when the served ball enters play (`BallEnteredPlay`), or on the first
///   ball in play switch hit, whichever comes first.
/// - Routine: `BallDraining` - Started for each drain (`BallsExitedPlay`), so its participants can save the ball. A
///   participant halting it saves the drain. A drain while another is being decided waits its turn.
/// - Routine: `TurnEnding` - Started once a drain that left no ball in play completes without being halted, or at once
///   on a tilt (`tilt`, `TurnEndReason::Tilted`). Modes clean up on its `RoutineStarted`. Once it completes and no ball
///   is left in play, the turn advances: to the same player, if they have an extra ball to use
///   (`TurnStartReason::ExtraBall`), otherwise to the next player.
///
/// Once the last turn's `TurnEnding` completes, or the game is quit (`quit_game`, also on a slam tilt), play is over:
/// - Routine: `GameEnding` - Started at the root. The game stays started while it runs, so its participants
///   (high-score entry, say) can take their time without a new game starting.
/// The game ends on `GameEnding`'s `RoutineCompleted`: the player groups are despawned and the states reset before
/// anyone else hears it, so systems that only run outside a game hear it too.
///
/// ## States
/// Set at the root, so every group reads them:
/// - `GamePhase` - Starting while `GameStarting` runs, then Active until play is over, then Ending while `GameEnding`
///   runs.
/// - `TurnPhase` - The current turn's Starting, Active, and Ending.
/// - `PlayerEntry` - Open while a player can be added: no game is started, or the first turn is still going and the
///   game isn't full.
///
/// ## Requires
/// - `BallManager` at the root
pub struct CompetitiveGame {
  /// This is the template to spin up a new group for the player
  systems_template: Vec<ChildSystemContainer>,
  max_players: u8,
  ball_in_play_switches: SwitchQ,
  game_state: Option<GameState>,
  /// The phase of the current turn. Kept here as well as set as a state, since a state reads as its old value until the
  /// run loop gets to it.
  turn_phase: TurnPhase,
  /// The group of each player added to the game, by player index
  player_groups: Vec<GroupHandle>,
  handle: SystemHandle,
  /// When the current turn became active, for `metrics::BALL_TIME`
  turn_active_at: Option<Instant>,
  /// Whether this turn's ball has been ejected. Until it has, nothing can make the turn active.
  ball_served: bool,
  /// Whether a `BallDraining` routine is running
  draining: bool,
  /// Drains waiting for the one being decided, oldest first
  pending_drains: VecDeque<BallDraining>,
  /// Whether the current turn was tilted. Points are ignored until the next turn.
  tilted: bool,
  /// Whether `TurnEnding` has completed, and the turn advances once no ball is left in play
  waiting_for_balls: bool,
  /// The phase of the game, kept here for the same reason as `turn_phase`. While it's `Starting` or `Ending`, no turn
  /// moves on.
  game_phase: GamePhase,
}

impl CompetitiveGame {
  pub fn new(
    max_players: u8,
    player_template: Vec<ChildSystemContainer>,
    ball_in_play_switches: SwitchQ,
  ) -> Self {
    Self {
      systems_template: player_template,
      max_players,
      ball_in_play_switches,
      game_state: None,
      turn_phase: TurnPhase::None,
      player_groups: Vec::new(),
      handle: SystemHandle::default(),
      turn_active_at: None,
      ball_served: false,
      draining: false,
      pending_drains: VecDeque::new(),
      tilted: false,
      waiting_for_balls: false,
      game_phase: GamePhase::None,
    }
  }

  fn player_group_name(player: u8) -> &'static str {
    PLAYER_GROUP_NAMES[player as usize]
  }

  fn start_game(&mut self, ctx: &ServiceContext) {
    let sys_ctx = ctx.for_system(self.handle);
    log::info!(target: "frontbox::game_manager", "Starting game with max players: {}", self.max_players);
    self.game_state = Some(GameState::competitive(self.max_players));
    self.set_game_phase(GamePhase::Starting, ctx);

    // Start a game assuming the number of balls in the trough is how many there should be
    // this avoids weird cases where a ball search may have abandoned a ball previously
    if let Some(mut ball_manager) = sys_ctx.get::<BallManager>() {
      ball_manager.establish_ball_count(ctx);
    }

    ctx.metrics.begin_game();
    ctx.metrics.inc(&metrics::GAMES_STARTED);
    sys_ctx.start_routine(GameStarting);
  }

  /// `GameStarting` completed: the first turn begins
  fn start_first_turn(&mut self, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Game started, beginning the first turn");
    self.set_game_phase(GamePhase::Active, ctx);
    self.start_turn(TurnStartReason::Ordered, ctx);
  }

  fn start_turn(&mut self, reason: TurnStartReason, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let player = self.game_state.as_ref().unwrap().current_player();

    ctx.activate_system_group(&self.player_groups[player as usize]);
    ctx.metrics.set_player(player);
    ctx.metrics.inc(&metrics::TURNS_PLAYED);

    log::info!(target: "frontbox::game_manager", "Transitioning current turn to starting");
    self.set_turn_phase(TurnPhase::Starting, ctx.into());
    self.update_player_entry(ctx.into());
    self.ball_served = false;
    self.draining = false;
    self.pending_drains.clear();
    self.tilted = false;
    self.waiting_for_balls = false;
    let game_state = self.game_state.as_ref().unwrap();
    let group = &self.player_groups[game_state.current_player() as usize];
    ctx.start_routine_in(
      group,
      TurnStarting {
        current_player: game_state.current_player(),
        turn: game_state.current_player_turn(),
        reason,
      },
    );

    ctx.emit(Synchronize);
  }

  fn set_turn_phase(&mut self, turn_phase: TurnPhase, ctx: &ServiceContext) {
    self.turn_phase = turn_phase;
    ctx.for_system(self.handle).set_state(turn_phase);
  }

  fn set_game_phase(&mut self, game_phase: GamePhase, ctx: &ServiceContext) {
    self.game_phase = game_phase;
    ctx.for_system(self.handle).set_state(game_phase);
  }

  fn is_game_started(&self) -> bool {
    self.game_state.is_some()
  }

  /// Whether a player can be added right now. Others read this as `PlayerEntry`, published by `update_player_entry`.
  fn is_player_addable(&self) -> bool {
    if self.game_phase == GamePhase::Ending {
      return false;
    }
    // if the game is started, then it must be the first player's turn on their first ball
    if let Some(game_state) = &self.game_state {
      return game_state.current_player() == 0
        && game_state.current_player_turn() == 0
        && game_state.player_count() < game_state.max_players();
    }

    true
  }

  /// Publishes `PlayerEntry`, from `is_player_addable`. Called after anything that could change it; setting it to the
  /// value it already has broadcasts nothing.
  fn update_player_entry(&self, ctx: &ServiceContext) {
    let entry = if self.is_player_addable() {
      PlayerEntry::Open
    } else {
      PlayerEntry::Closed
    };
    ctx.for_system(self.handle).set_state(entry);
  }

  fn serve_ball(&mut self, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Serving the ball");
    self.ball_served = true;
    let ctx = &ctx.for_system(self.handle);
    if let Some(mut ball_manager) = ctx.get::<BallManager>() {
      ball_manager.eject(Launch::Player, ctx.into());
    }
  }

  fn transition_turn_to_active(&mut self, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Transitioning current turn to active");
    self.set_turn_phase(TurnPhase::Active, ctx);
    let game_state = self.game_state.as_ref().unwrap();
    self.turn_active_at = Some(Instant::now());
    ctx.for_system(self.handle).emit(TurnActive::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  /// Balls left play. The routine's participants decide whether the drain stands; one that's already being decided
  /// goes first, as only one routine of a type runs at a time.
  fn drained(&mut self, count: u8, in_play: u8, ctx: &ServiceContext) {
    let game_state = self.game_state.as_ref().unwrap();
    let draining = BallDraining {
      current_player: game_state.current_player(),
      turn: game_state.current_player_turn(),
      count,
      in_play,
    };
    if self.draining {
      log::info!(target: "frontbox::game_manager", "{} ball(s) drained while deciding another drain, waiting", count);
      self.pending_drains.push_back(draining);
    } else {
      self.start_ball_draining(draining, ctx);
    }
  }

  fn start_ball_draining(&mut self, draining: BallDraining, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "{} ball(s) drained, {} left in play", draining.count, draining.in_play);
    self.draining = true;
    let group = &self.player_groups[draining.current_player as usize];
    ctx
      .for_system(self.handle)
      .start_routine_in(group, draining);
  }

  fn drain_decided(&mut self, done: &RoutineCompleted<BallDraining>, ctx: &ServiceContext) {
    self.draining = false;
    match done.completion {
      RoutineCompletion::Halted => {
        log::info!(target: "frontbox::game_manager", "Drain was saved")
      }
      RoutineCompletion::AllSteps if done.routine.in_play == 0 => {
        self.pending_drains.clear();
        self.transition_turn_to_ending(TurnEndReason::Drained, ctx);
        return;
      }
      RoutineCompletion::AllSteps => {}
    }
    if let Some(next) = self.pending_drains.pop_front() {
      self.start_ball_draining(next, ctx);
    }
  }

  fn transition_turn_to_ending(&mut self, reason: TurnEndReason, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Transitioning current turn to ending ({:?})", reason);
    self.set_turn_phase(TurnPhase::Ending, ctx);
    if let Some(active_at) = self.turn_active_at.take() {
      ctx.metrics.record(&metrics::BALL_TIME, active_at.elapsed());
    }
    let game_state = self.game_state.as_ref().unwrap();
    let group = &self.player_groups[game_state.current_player() as usize];
    ctx.for_system(self.handle).start_routine_in(
      group,
      TurnEnding {
        current_player: game_state.current_player(),
        turn: game_state.current_player_turn(),
        reason,
      },
    );
  }

  /// `TurnEnding` completed. The turn advances once no ball is left in play: balls released during the routine, or
  /// still rolling down after a tilt, are waited for.
  fn turn_ended(&mut self, ctx: &ServiceContext) {
    let in_play = self.balls_in_play(ctx);
    if in_play > 0 {
      log::info!(target: "frontbox::game_manager", "Waiting for {} ball(s) to leave play before advancing", in_play);
      self.waiting_for_balls = true;
    } else {
      self.advance_turn(ctx);
    }
  }

  fn balls_in_play(&self, ctx: &ServiceContext) -> u8 {
    let ctx = &ctx.for_system(self.handle);
    ctx
      .get::<BallManager>()
      .map(|ball_manager| ball_manager.balls_in_play(ctx.into()))
      .unwrap_or(0)
  }

  fn advance_turn(&mut self, ctx: &ServiceContext) {
    self.waiting_for_balls = false;
    let max_turn_count = ctx.config.get(&configs::TURN_COUNT);
    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!(target: "frontbox::game_manager", "Game state should have been initialized when starting the game");
      return;
    };

    let player = game_state.current_player();
    if game_state.use_extra_ball(player) {
      let remaining = game_state.extra_balls(player);
      log::info!(target: "frontbox::game_manager", "Using an extra ball, {} unused", remaining);
      ctx.metrics.inc(&metrics::EXTRA_BALLS);
      ctx.metrics.flush();
      ctx.for_system(self.handle).emit(ExtraBallUsed {
        player_index: player,
        remaining,
      });
      // the player's group stays active, as they're up again
      self.start_turn(TurnStartReason::ExtraBall, ctx);
      return;
    }

    ctx.deactivate_system_group(&self.player_groups[player as usize]);
    ctx.metrics.flush();

    // increment the previous player's turn before advancing to next player/turn
    game_state.increment_current_player_turn();
    game_state.advance_turn();

    // Verify we haven't gone over the turn limit
    if game_state.current_player_turn() >= max_turn_count {
      self.start_game_ending(GameEndReason::Completed, ctx);
      return;
    }

    log::info!(
      target: "frontbox::game_manager",
      "Advancing turn (player: {}, turn: {})",
      game_state.current_player(),
      game_state.current_player_turn()
    );

    self.start_turn(TurnStartReason::Ordered, ctx);
  }

  /// Play is over. The game stays started until `GameEnding` completes.
  fn start_game_ending(&mut self, reason: GameEndReason, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Game ending ({:?})", reason);
    self.set_game_phase(GamePhase::Ending, ctx);
    self.update_player_entry(ctx);
    self.draining = false;
    self.pending_drains.clear();
    self.waiting_for_balls = false;
    self.turn_active_at = None;

    let scores = self.scores();
    for (player, (_, score)) in scores.iter().enumerate() {
      ctx
        .metrics
        .record_for_player(&metrics::FINAL_SCORE, player as u8, *score);
    }
    ctx.metrics.end_game();

    ctx
      .for_system(self.handle)
      .start_routine(GameEnding { scores, reason });
  }

  /// `GameEnding` completed: the game is over
  fn end_game(&mut self, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Game ended");
    for group in self.player_groups.drain(..) {
      ctx.despawn_system_group(group);
    }
    self.game_state = None;
    self.set_game_phase(GamePhase::None, ctx);
    self.set_turn_phase(TurnPhase::None, ctx);
    self.update_player_entry(ctx);
  }

  /// Each player's score, in player order
  fn scores(&self) -> Vec<(&'static str, u32)> {
    match &self.game_state {
      Some(GameState::Competitive {
        player_scores,
        player_count,
        ..
      }) => player_scores
        .iter()
        .take(*player_count as usize)
        .enumerate()
        .map(|(idx, score)| (PLAYER_GROUP_NAMES[idx], *score))
        .collect(),
      _ => Vec::new(),
    }
  }
}

impl System for CompetitiveGame {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
    self.update_player_entry(ctx.into());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    match self.game_phase {
      GamePhase::Starting => {
        if event.is::<RoutineCompleted<GameStarting>>() {
          self.start_first_turn(ctx.into());
        }
        return;
      }
      GamePhase::Ending => {
        // routines still running after a quit complete on their own, and are ignored
        if event.is::<RoutineCompleted<GameEnding>>() {
          self.end_game(ctx.into());
        }
        return;
      }
      GamePhase::None | GamePhase::Active => {}
    }

    match self.turn_phase {
      TurnPhase::None => {}
      TurnPhase::Starting if !self.ball_served => {
        if event.is::<RoutineCompleted<TurnStarting>>() {
          self.serve_ball(ctx.into());
        }
      }
      TurnPhase::Starting => {
        if event.is::<BallEnteredPlay>() {
          log::debug!(target: "frontbox::game_manager", "Active caused by ball entering play");
          self.transition_turn_to_active(ctx.into());
        } else if let Some(e) = event.downcast_ref::<SwitchClosed>()
          && self.ball_in_play_switches.matches(&e.switch)
        {
          log::debug!(target: "frontbox::game_manager", "Active caused by switch: {}", e.switch.name);
          self.transition_turn_to_active(ctx.into());
        }
      }
      TurnPhase::Active => {
        if let Some(BallsExitedPlay { count, in_play }) = event.downcast_ref::<BallsExitedPlay>() {
          self.drained(*count, *in_play, ctx.into());
        } else if let Some(done) = event.downcast_ref::<RoutineCompleted<BallDraining>>() {
          self.drain_decided(done, ctx.into());
        }
      }
      TurnPhase::Ending => {
        // drains while the turn is ending (balls rolling down after a tilt) start no BallDraining: they can't be saved
        if event.is::<RoutineCompleted<TurnEnding>>() {
          self.turn_ended(ctx.into());
        } else if let Some(BallsExitedPlay { in_play: 0, .. }) =
          event.downcast_ref::<BallsExitedPlay>()
          && self.waiting_for_balls
        {
          self.advance_turn(ctx.into());
        }
      }
    }
  }
}

impl GameManagement for CompetitiveGame {
  fn add_player(&mut self, ctx: &ServiceContext) {
    // checked here as well as through `PlayerEntry`, which reads as its old value until the run loop gets to it
    if !self.is_player_addable() {
      log::warn!(target: "frontbox::game_manager", "A player can't be added now, ignored");
      return;
    }
    if !self.is_game_started() {
      self.start_game(ctx);
    }

    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!(target: "frontbox::game_manager", "Game state should have been initialized when starting the game");
      return;
    };

    game_state.increment_player_count();
    log::info!(
      target: "frontbox::game_manager",
      "Adding player to game (current count: {})",
      game_state.player_count()
    );

    // create copy of systems for new player as a new system group
    let copy = self.systems_template.to_vec();
    let group_name = Self::player_group_name(game_state.player_count() - 1);

    let svc_ctx = ctx.for_system(self.handle);
    self
      .player_groups
      .push(svc_ctx.spawn_system_group(group_name, copy, false));
    svc_ctx.emit(PlayerAdded);
    self.update_player_entry(ctx);
  }

  fn quit_game(&mut self, reason: GameEndReason, ctx: &ServiceContext) {
    let Some(player) = self.game_state.as_ref().map(GameState::current_player) else {
      return;
    };
    if self.game_phase == GamePhase::Ending {
      log::info!(target: "frontbox::game_manager", "Quit while the game is already ending, ignored");
      return;
    }

    log::info!(target: "frontbox::game_manager", "Quitting game ({:?})", reason);
    if reason == GameEndReason::Quit {
      ctx.metrics.inc(&metrics::GAMES_QUIT);
    }
    let ctx = &ctx.for_system(self.handle);
    // quit during GameStarting: no turn has begun, so there's none to end
    if self.game_phase == GamePhase::Active {
      // the turn is over without a TurnEnding; its group's systems stop as it deactivates
      self.set_turn_phase(TurnPhase::Ending, ctx.into());
      ctx.deactivate_system_group(&self.player_groups[player as usize]);
      ctx.metrics.flush();
    }
    self.start_game_ending(reason, ctx.into());
  }

  fn tilt(&mut self, ctx: &ServiceContext) {
    let playing = match self.turn_phase {
      TurnPhase::Starting => self.ball_served,
      TurnPhase::Active => true,
      TurnPhase::None | TurnPhase::Ending => false,
    };
    if self.game_phase != GamePhase::Active || !playing {
      log::info!(target: "frontbox::game_manager", "Tilt while no ball is being played, ignored");
      return;
    }
    let Some(player) = self.game_state.as_ref().map(GameState::current_player) else {
      return;
    };

    log::info!(target: "frontbox::game_manager", "Player {} tilted", player + 1);
    ctx.metrics.inc(&metrics::TILTS);
    self.tilted = true;
    // a drain being decided completes on its own, and is ignored as the turn is ending
    self.draining = false;
    self.pending_drains.clear();
    ctx.for_system(self.handle).emit(PlayerTilted {
      player_index: player,
    });
    self.transition_turn_to_ending(TurnEndReason::Tilted, ctx);
  }

  fn game_state(&self) -> Option<&GameState> {
    self.game_state.as_ref()
  }

  fn clear_multiplier(&mut self) {
    if let Some(game_state) = &mut self.game_state
      && let Some(multiplier) = game_state.current_player_multiplier_mut()
    {
      *multiplier = 1.0;
    }
  }

  fn set_multiplier(&mut self, multiplier: f32) {
    if let Some(game_state) = &mut self.game_state
      && let Some(current_multiplier) = game_state.current_player_multiplier_mut()
    {
      *current_multiplier = multiplier;
    }
  }

  fn add_points(&mut self, points: u32, ctx: &ServiceContext) {
    if self.tilted {
      log::debug!(target: "frontbox::game_manager", "{} points ignored, the turn was tilted", points);
      return;
    }
    if let Some(game_state) = &mut self.game_state {
      let multiplier = game_state.current_player_multiplier();
      let points_received = (points as f32 * multiplier) as u32;
      if let Some(score) = game_state.player_score_mut(game_state.current_player()) {
        *score += points_received;
        let total_points = *score;

        // Emit event with points added information
        ctx.for_system(self.handle).emit(PointsAdded::new(
          game_state.current_player(),
          points_received,
          total_points,
        ));
      }
    }
  }

  fn grant_extra_ball(&mut self, ctx: &ServiceContext) -> bool {
    if self.game_phase != GamePhase::Active {
      log::warn!(target: "frontbox::game_manager", "An extra ball can only be granted during play, ignored");
      return false;
    }
    let Some(game_state) = &mut self.game_state else {
      return false;
    };

    let player = game_state.current_player();
    let max = ctx.config.get(&configs::MAX_EXTRA_BALLS);
    if game_state.extra_balls_granted(player) >= max {
      log::info!(target: "frontbox::game_manager", "Extra ball limit of {} reached, not granted", max);
      return false;
    }

    game_state.grant_extra_ball(player);
    let count = game_state.extra_balls(player);
    log::info!(target: "frontbox::game_manager", "Extra ball granted, {} unused", count);
    ctx.for_system(self.handle).emit(ExtraBallGranted {
      player_index: player,
      count,
    });
    true
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU8, Ordering};

  use super::*;
  use crate::ball_management::{BallManagement, LaunchMethod};

  fn done<E: Event>(routine: E, completion: RoutineCompletion) -> RoutineCompleted<E> {
    RoutineCompleted {
      routine,
      completion,
    }
  }

  struct Fixture {
    game: CompetitiveGame,
    ctx: TestContext,
    /// What `FakeBalls` reports as in play
    in_play: Arc<AtomicU8>,
  }

  /// Ball management that only reports the balls in play it's told
  struct FakeBalls {
    in_play: Arc<AtomicU8>,
  }

  impl System for FakeBalls {}

  impl BallManagement for FakeBalls {
    fn eject(&mut self, _launch: Launch, _ctx: &ServiceContext) -> u64 {
      0
    }
    fn eject_many(&mut self, _count: u8, _launch: Launch, _ctx: &ServiceContext) -> u64 {
      0
    }
    fn can_auto_launch(&self) -> bool {
      true
    }
    fn balls_in_play(&self, _ctx: &ServiceContext) -> u8 {
      self.in_play.load(Ordering::SeqCst)
    }
    fn establish_ball_count(&mut self, _ctx: &ServiceContext) {}
    fn ball_temporarily_removed_from_play(&mut self, _ctx: &ServiceContext) {}
    fn ball_returned_to_play(&mut self, _ctx: &ServiceContext) {}
  }

  impl Fixture {
    /// A one-player game of `turns` turns, with the first turn's ball in play
    fn playing(turns: u8) -> Self {
      let mut f = Self::started(turns);
      f.event(done(
        TurnStarting {
          current_player: 0,
          turn: 0,
          reason: TurnStartReason::Ordered,
        },
        RoutineCompletion::AllSteps,
      ));
      f.event(BallEnteredPlay {
        method: LaunchMethod::Manual,
        in_play: 1,
      });
      f.ctx.routines_started();
      f
    }

    /// A one-player game of `turns` turns, with the first turn's `TurnStarting` running
    fn started(turns: u8) -> Self {
      let mut f = Self::starting(1, turns);
      f.event(done(GameStarting, RoutineCompletion::AllSteps));
      f
    }

    /// A game of up to `max_players` players and `turns` turns, with one player added and `GameStarting` running
    fn starting(max_players: u8, turns: u8) -> Self {
      let mut ctx = TestContext::default();
      ctx.svc_ctx().config.set(&configs::TURN_COUNT, turns);
      let in_play = Arc::new(AtomicU8::new(0));
      ctx.insert_system(BallManager::new(FakeBalls {
        in_play: in_play.clone(),
      }));
      let mut game = CompetitiveGame::new(max_players, Vec::new(), SwitchQ::names([]));
      game.on_spawn(&ctx.sys_ctx());
      game.add_player(&ctx.svc_ctx());
      Self { game, ctx, in_play }
    }

    fn event(&mut self, event: impl Event) {
      self.game.on_event(&event, &self.ctx.sys_ctx());
    }

    /// The ball drains without being saved, and the turn's `TurnEnding` completes
    fn play_out_turn(&mut self) {
      let state = self.game.game_state().unwrap();
      let (current_player, turn) = (state.current_player(), state.current_player_turn());
      self.event(BallsExitedPlay {
        count: 1,
        in_play: 0,
      });
      self.event(done(
        BallDraining {
          current_player,
          turn,
          count: 1,
          in_play: 0,
        },
        RoutineCompletion::AllSteps,
      ));
      self.complete_turn_ending(TurnEndReason::Drained);
    }

    fn complete_turn_ending(&mut self, reason: TurnEndReason) {
      let state = self.game.game_state().unwrap();
      let (current_player, turn) = (state.current_player(), state.current_player_turn());
      self.event(done(
        TurnEnding {
          current_player,
          turn,
          reason,
        },
        RoutineCompletion::AllSteps,
      ));
    }

    /// Balls in play, as ball management reports them
    fn set_in_play(&mut self, in_play: u8) {
      self.in_play.store(in_play, Ordering::SeqCst);
    }

    /// The reasons of the `TurnEnding` routines started since the last check
    fn turn_endings(&mut self) -> Vec<TurnEndReason> {
      self
        .ctx
        .routines_started()
        .iter()
        .filter_map(|routine| routine.downcast_ref::<TurnEnding>())
        .map(|t| t.reason)
        .collect()
    }

    fn score(&self) -> u32 {
      self.game.scores()[0].1
    }

    /// The `GameEnding` routines started since the last check
    fn game_endings(&mut self) -> Vec<GameEnding> {
      self
        .ctx
        .routines_started()
        .iter()
        .filter_map(|routine| routine.downcast_ref::<GameEnding>().cloned())
        .collect()
    }

    /// The `TurnStarting` routines started since the last check, as (player, turn, reason)
    fn turn_startings(&mut self) -> Vec<(u8, u8, TurnStartReason)> {
      self
        .ctx
        .routines_started()
        .iter()
        .filter_map(|routine| routine.downcast_ref::<TurnStarting>())
        .map(|t| (t.current_player, t.turn, t.reason))
        .collect()
    }

    /// The ball of the turn just started enters play
    fn serve(&mut self) {
      let state = self.game.game_state().unwrap();
      let (current_player, turn) = (state.current_player(), state.current_player_turn());
      self.event(done(
        TurnStarting {
          current_player,
          turn,
          reason: TurnStartReason::Ordered,
        },
        RoutineCompletion::AllSteps,
      ));
      self.event(BallEnteredPlay {
        method: LaunchMethod::Manual,
        in_play: 1,
      });
    }
  }

  #[test]
  fn an_extra_ball_starts_another_turn_for_the_same_player() {
    let mut f = Fixture::playing(1);
    assert!(f.game.grant_extra_ball(&f.ctx.svc_ctx()));
    f.play_out_turn();

    assert_eq!(
      f.game.game_phase,
      GamePhase::Active,
      "the last turn's extra ball is still played"
    );
    assert_eq!(
      f.turn_startings(),
      vec![(0, 0, TurnStartReason::ExtraBall)],
      "same player, same turn number"
    );
    assert_eq!(f.game.turn_phase, TurnPhase::Starting);
    assert_eq!(f.game.game_state().unwrap().extra_balls(0), 0);

    f.serve();
    f.play_out_turn();
    assert_eq!(f.game_endings().len(), 1, "no extra ball left");
  }

  #[test]
  fn granting_and_using_extra_balls_emit_events() {
    let mut f = Fixture::playing(1);
    f.game.grant_extra_ball(&f.ctx.svc_ctx());
    f.game.grant_extra_ball(&f.ctx.svc_ctx());
    f.play_out_turn();

    let events = f.ctx.events_emitted();
    let granted: Vec<_> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<ExtraBallGranted>())
      .map(|e| (e.player_index, e.count))
      .collect();
    let used: Vec<_> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<ExtraBallUsed>())
      .map(|e| (e.player_index, e.remaining))
      .collect();
    assert_eq!(granted, vec![(0, 1), (0, 2)]);
    assert_eq!(used, vec![(0, 1)]);
  }

  #[test]
  fn an_extra_ball_keeps_the_same_player_up() {
    let mut f = Fixture::starting(2, 3);
    f.game.add_player(&f.ctx.svc_ctx());
    f.event(done(GameStarting, RoutineCompletion::AllSteps));
    f.serve();
    f.ctx.routines_started();

    f.game.grant_extra_ball(&f.ctx.svc_ctx());
    f.play_out_turn();
    assert_eq!(f.turn_startings(), vec![(0, 0, TurnStartReason::ExtraBall)]);

    f.serve();
    f.play_out_turn();
    assert_eq!(f.turn_startings(), vec![(1, 0, TurnStartReason::Ordered)]);
  }

  #[test]
  fn an_extra_ball_granted_while_turn_ending_runs_is_used_for_the_next_turn() {
    let mut f = Fixture::playing(1);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    f.event(done(
      BallDraining {
        current_player: 0,
        turn: 0,
        count: 1,
        in_play: 0,
      },
      RoutineCompletion::AllSteps,
    ));
    assert_eq!(f.game.turn_phase, TurnPhase::Ending);

    assert!(f.game.grant_extra_ball(&f.ctx.svc_ctx()));
    f.complete_turn_ending(TurnEndReason::Drained);
    assert!(f.game_endings().is_empty());
    assert_eq!(f.game.turn_phase, TurnPhase::Starting);
  }

  #[test]
  fn extra_balls_are_limited_per_game() {
    let mut f = Fixture::playing(3);
    f.ctx.svc_ctx().config.set(&configs::MAX_EXTRA_BALLS, 1);
    assert!(f.game.grant_extra_ball(&f.ctx.svc_ctx()));
    assert!(!f.game.grant_extra_ball(&f.ctx.svc_ctx()));
    assert_eq!(f.game.game_state().unwrap().extra_balls(0), 1);

    f.play_out_turn();
    f.serve();
    assert!(
      !f.game.grant_extra_ball(&f.ctx.svc_ctx()),
      "using one doesn't make room for another"
    );
  }

  #[test]
  fn extra_balls_cannot_be_granted_before_the_first_turn() {
    let mut f = Fixture::starting(1, 3);
    assert!(!f.game.grant_extra_ball(&f.ctx.svc_ctx()));
  }

  #[test]
  fn the_last_turn_starts_game_ending() {
    let mut f = Fixture::playing(1);
    f.game.add_points(100, &f.ctx.svc_ctx());
    f.play_out_turn();

    let endings = f.game_endings();
    assert_eq!(endings.len(), 1);
    assert_eq!(endings[0].reason, GameEndReason::Completed);
    assert_eq!(endings[0].scores, vec![("player1", 100)]);
  }

  #[test]
  fn starting_a_game_starts_game_starting() {
    let mut f = Fixture::starting(1, 1);
    let started = f.ctx.routines_started();
    assert_eq!(started.len(), 1);
    assert!(started[0].is::<GameStarting>());
    assert_eq!(f.game.game_phase, GamePhase::Starting);
    assert_eq!(f.game.turn_phase, TurnPhase::None);
  }

  #[test]
  fn the_first_turn_starts_once_game_starting_completes() {
    let mut f = Fixture::starting(1, 1);
    f.ctx.routines_started();

    f.event(done(GameStarting, RoutineCompletion::AllSteps));
    assert_eq!(f.game.game_phase, GamePhase::Active);
    assert_eq!(f.game.turn_phase, TurnPhase::Starting);
    let started = f.ctx.routines_started();
    assert_eq!(started.len(), 1);
    assert!(started[0].is::<TurnStarting>());
  }

  #[test]
  fn players_can_be_added_while_game_starting_runs() {
    let mut f = Fixture::starting(2, 1);
    assert!(f.game.is_player_addable());
    f.game.add_player(&f.ctx.svc_ctx());
    assert_eq!(f.game.game_state().unwrap().player_count(), 2);
    assert!(
      f.ctx
        .routines_started()
        .iter()
        .all(|routine| !routine.is::<TurnStarting>()),
      "no turn starts while GameStarting runs"
    );
  }

  #[test]
  fn quitting_while_game_starting_runs_starts_game_ending() {
    let mut f = Fixture::starting(1, 3);
    f.game.quit_game(GameEndReason::Quit, &f.ctx.svc_ctx());

    let endings = f.game_endings();
    assert_eq!(endings.len(), 1);
    assert_eq!(endings[0].reason, GameEndReason::Quit);
    assert_eq!(f.game.turn_phase, TurnPhase::None);

    f.event(done(GameStarting, RoutineCompletion::AllSteps));
    assert!(
      f.ctx
        .routines_started()
        .iter()
        .all(|routine| !routine.is::<TurnStarting>()),
      "GameStarting completing after a quit is ignored"
    );
  }

  #[test]
  fn the_game_is_ending_while_game_ending_runs() {
    let mut f = Fixture::playing(1);
    f.play_out_turn();
    assert_eq!(f.game.game_phase, GamePhase::Ending);

    let ending = f.game_endings().remove(0);
    f.event(done(ending, RoutineCompletion::AllSteps));
    assert_eq!(f.game.game_phase, GamePhase::None);
    assert_eq!(f.game.turn_phase, TurnPhase::None);
  }

  #[test]
  fn the_game_stays_started_until_game_ending_completes() {
    let mut f = Fixture::playing(1);
    f.play_out_turn();
    let ending = f.game_endings().remove(0);
    assert!(f.game.is_game_started());
    assert!(!f.game.is_player_addable(), "no new game during GameEnding");

    f.event(done(ending, RoutineCompletion::AllSteps));
    assert!(!f.game.is_game_started());
    assert!(f.game.is_player_addable());
  }

  #[test]
  fn adding_a_player_while_the_game_is_ending_is_ignored() {
    let mut f = Fixture::playing(1);
    f.play_out_turn();

    f.game.add_player(&f.ctx.svc_ctx());
    assert_eq!(f.game.game_state().unwrap().player_count(), 1);
  }

  #[test]
  fn turns_before_the_last_do_not_end_the_game() {
    let mut f = Fixture::playing(2);
    f.play_out_turn();
    assert!(f.game_endings().is_empty());
  }

  #[test]
  fn quitting_mid_turn_starts_game_ending() {
    let mut f = Fixture::playing(3);
    f.game.quit_game(GameEndReason::Quit, &f.ctx.svc_ctx());

    let endings = f.game_endings();
    assert_eq!(endings.len(), 1);
    assert_eq!(endings[0].reason, GameEndReason::Quit);
    assert_eq!(f.game.turn_phase, TurnPhase::Ending);
  }

  #[test]
  fn the_turn_is_ignored_once_the_game_is_ending() {
    let mut f = Fixture::playing(3);
    f.game.quit_game(GameEndReason::Quit, &f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert!(f.ctx.routines_started().is_empty(), "no BallDraining");
  }

  #[test]
  fn quitting_while_the_game_is_ending_is_ignored() {
    let mut f = Fixture::playing(1);
    f.play_out_turn();
    f.game_endings();

    f.game.quit_game(GameEndReason::Quit, &f.ctx.svc_ctx());
    assert!(f.game_endings().is_empty());
  }

  #[test]
  fn a_drain_while_another_is_being_decided_waits_its_turn() {
    let mut f = Fixture::playing(3);
    let draining = |count, in_play| BallDraining {
      current_player: 0,
      turn: 0,
      count,
      in_play,
    };
    let started = |f: &mut Fixture| -> Vec<u8> {
      f.ctx
        .routines_started()
        .iter()
        .filter_map(|routine| routine.downcast_ref::<BallDraining>().map(|d| d.in_play))
        .collect()
    };

    f.event(BallsExitedPlay {
      count: 1,
      in_play: 1,
    });
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert_eq!(started(&mut f), vec![1], "the second drain waits");

    f.event(done(draining(1, 1), RoutineCompletion::Halted));
    assert_eq!(started(&mut f), vec![0]);

    f.event(done(draining(1, 0), RoutineCompletion::AllSteps));
    assert_eq!(f.game.turn_phase, TurnPhase::Ending);
  }

  #[test]
  fn a_tilt_starts_turn_ending_at_once() {
    let mut f = Fixture::playing(3);
    f.set_in_play(1);
    f.game.tilt(&f.ctx.svc_ctx());

    assert_eq!(f.game.turn_phase, TurnPhase::Ending);
    assert_eq!(f.turn_endings(), vec![TurnEndReason::Tilted]);
  }

  #[test]
  fn a_tilt_emits_player_tilted() {
    let mut f = Fixture::playing(3);
    f.game.tilt(&f.ctx.svc_ctx());

    let tilted: Vec<_> = f
      .ctx
      .events_emitted()
      .iter()
      .filter_map(|e| e.event.downcast_ref::<PlayerTilted>())
      .map(|e| e.player_index)
      .collect();
    assert_eq!(tilted, vec![0]);
  }

  #[test]
  fn a_tilt_once_the_ball_is_served_counts() {
    let mut f = Fixture::started(3);
    f.event(done(
      TurnStarting {
        current_player: 0,
        turn: 0,
        reason: TurnStartReason::Ordered,
      },
      RoutineCompletion::AllSteps,
    ));
    assert_eq!(
      f.game.turn_phase,
      TurnPhase::Starting,
      "the ball isn't in play yet"
    );

    f.game.tilt(&f.ctx.svc_ctx());
    assert_eq!(f.game.turn_phase, TurnPhase::Ending);
  }

  #[test]
  fn a_tilt_before_the_ball_is_served_is_ignored() {
    let mut f = Fixture::started(3);
    f.game.tilt(&f.ctx.svc_ctx());
    assert_eq!(f.game.turn_phase, TurnPhase::Starting);
  }

  #[test]
  fn a_second_tilt_is_ignored() {
    let mut f = Fixture::playing(3);
    f.game.tilt(&f.ctx.svc_ctx());
    f.game.tilt(&f.ctx.svc_ctx());
    assert_eq!(f.turn_endings().len(), 1);
  }

  #[test]
  fn points_are_ignored_for_the_rest_of_a_tilted_turn() {
    let mut f = Fixture::playing(3);
    f.game.add_points(100, &f.ctx.svc_ctx());
    f.game.tilt(&f.ctx.svc_ctx());
    f.game.add_points(50, &f.ctx.svc_ctx());
    assert_eq!(f.score(), 100);

    f.complete_turn_ending(TurnEndReason::Tilted);
    f.serve();
    f.game.add_points(10, &f.ctx.svc_ctx());
    assert_eq!(f.score(), 110, "the next turn scores");
  }

  #[test]
  fn drains_after_a_tilt_cannot_be_saved() {
    let mut f = Fixture::playing(3);
    f.game.tilt(&f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert!(f.ctx.routines_started().is_empty(), "no BallDraining");
  }

  #[test]
  fn a_drain_being_decided_when_tilting_is_ignored() {
    let mut f = Fixture::playing(3);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    f.game.tilt(&f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.event(done(
      BallDraining {
        current_player: 0,
        turn: 0,
        count: 1,
        in_play: 0,
      },
      RoutineCompletion::AllSteps,
    ));
    assert!(f.turn_endings().is_empty(), "the turn is already ending");
  }

  #[test]
  fn the_turn_advances_once_the_balls_left_after_a_tilt_drain() {
    let mut f = Fixture::playing(3);
    f.set_in_play(2);
    f.game.tilt(&f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.complete_turn_ending(TurnEndReason::Tilted);
    assert!(
      f.turn_startings().is_empty(),
      "balls are still rolling down"
    );

    f.set_in_play(1);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 1,
    });
    assert!(f.turn_startings().is_empty());

    f.set_in_play(0);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert_eq!(f.turn_startings(), vec![(0, 1, TurnStartReason::Ordered)]);
  }

  #[test]
  fn a_ball_released_during_turn_ending_is_waited_for() {
    let mut f = Fixture::playing(3);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    f.event(done(
      BallDraining {
        current_player: 0,
        turn: 0,
        count: 1,
        in_play: 0,
      },
      RoutineCompletion::AllSteps,
    ));
    f.ctx.routines_started();

    // a mode holding a ball lets it go as the turn ends
    f.set_in_play(1);
    f.complete_turn_ending(TurnEndReason::Drained);
    assert!(f.turn_startings().is_empty());

    f.set_in_play(0);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert_eq!(f.turn_startings(), vec![(0, 1, TurnStartReason::Ordered)]);
  }

  #[test]
  fn drains_before_turn_ending_completes_do_not_advance_the_turn() {
    let mut f = Fixture::playing(3);
    f.set_in_play(1);
    f.game.tilt(&f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.set_in_play(0);
    f.event(BallsExitedPlay {
      count: 1,
      in_play: 0,
    });
    assert!(f.turn_startings().is_empty(), "TurnEnding is still running");

    f.complete_turn_ending(TurnEndReason::Tilted);
    assert_eq!(f.turn_startings(), vec![(0, 1, TurnStartReason::Ordered)]);
  }

  #[test]
  fn a_tilted_turn_still_uses_an_extra_ball() {
    let mut f = Fixture::playing(3);
    f.game.grant_extra_ball(&f.ctx.svc_ctx());
    f.game.tilt(&f.ctx.svc_ctx());
    f.ctx.routines_started();

    f.complete_turn_ending(TurnEndReason::Tilted);
    assert_eq!(f.turn_startings(), vec![(0, 0, TurnStartReason::ExtraBall)]);
  }

  #[test]
  fn a_slam_tilt_ends_the_game() {
    let mut f = Fixture::playing(3);
    f.game
      .quit_game(GameEndReason::SlamTilted, &f.ctx.svc_ctx());

    let endings = f.game_endings();
    assert_eq!(endings.len(), 1);
    assert_eq!(endings[0].reason, GameEndReason::SlamTilted);
  }
}
