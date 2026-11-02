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
/// Each turn goes through these, in order. The routines are started in the current player's group, so their
/// participants can be in the player template as well as at the root.
/// - Routine: `TurnStarting` - Started at the start of a player's turn, once their group is active. The ball is ejected
///   on its `RoutineCompleted`, so participants can hold the serve (e.g. a player-up display).
/// - Event: `TurnActive` - Emitted once per turn, when the served ball enters play (`BallEnteredPlay`), or on the first
///   ball in play switch hit, whichever comes first.
/// - Routine: `BallDraining` - Started for each drain (`BallsExitedPlay`), so its participants can save the ball. A
///   participant halting it saves the drain. A drain while another is being decided waits its turn.
/// - Routine: `TurnEnding` - Started once a drain that left no ball in play completes without being halted. Modes
///   clean up on its `RoutineStarted`, and the turn advances on its `RoutineCompleted`.
///
/// Once the last turn's `TurnEnding` completes, or the game is quit (`quit_game`), play is over:
/// - Routine: `GameEnding` - Started at the root. The game stays started while it runs, so its participants
///   (high-score entry, say) can take their time without a new game starting.
/// - Event: `GameEnded` - Emitted once `GameEnding` completes and the player groups are gone.
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
  turn_state: TurnState,
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
  /// Whether `GameEnding` is running. Nothing else about the game moves on meanwhile.
  game_ending: bool,
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
      turn_state: TurnState::Idle,
      player_groups: Vec::new(),
      handle: SystemHandle::default(),
      turn_active_at: None,
      ball_served: false,
      draining: false,
      pending_drains: VecDeque::new(),
      game_ending: false,
    }
  }

  fn player_group_name(player: u8) -> &'static str {
    PLAYER_GROUP_NAMES[player as usize]
  }

  fn start_game(&mut self, ctx: &ServiceContext) {
    let sys_ctx = ctx.for_system(self.handle);
    log::info!(target: "frontbox::game_manager", "Starting game with max players: {}", self.max_players);
    self.game_state = Some(GameState::competitive(self.max_players));

    // Start a game assuming the number of balls in the trough is how many there should be
    // this avoids weird cases where a ball search may have abandoned a ball previously
    if let Some(mut ball_manager) = sys_ctx.get::<BallManager>() {
      ball_manager.establish_ball_count(ctx);
    }

    ctx.metrics.begin_game();
    ctx.metrics.inc(&metrics::GAMES_STARTED);
    sys_ctx.emit(GameStarted);
  }

  fn start_turn(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let player = self.game_state.as_ref().unwrap().current_player();

    ctx.activate_system_group(&self.player_groups[player as usize]);
    ctx.metrics.set_player(player);
    ctx.metrics.inc(&metrics::TURNS_PLAYED);

    log::info!(target: "frontbox::game_manager", "Transitioning current turn to starting");
    self.set_turn_state(TurnState::Starting, ctx.into());
    self.ball_served = false;
    self.draining = false;
    self.pending_drains.clear();
    let game_state = self.game_state.as_ref().unwrap();
    let group = &self.player_groups[game_state.current_player() as usize];
    ctx.start_routine_in(
      group,
      TurnStarting {
        current_player: game_state.current_player(),
        turn: game_state.current_player_turn(),
      },
    );

    ctx.emit(Synchronize);
  }

  fn set_turn_state(&mut self, turn_state: TurnState, ctx: &ServiceContext) {
    self.turn_state = turn_state;
    ctx.for_system(self.handle).set_state(turn_state);
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
    self.set_turn_state(TurnState::Active, ctx);
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
        self.transition_turn_to_ending(ctx);
        return;
      }
      RoutineCompletion::AllSteps => {}
    }
    if let Some(next) = self.pending_drains.pop_front() {
      self.start_ball_draining(next, ctx);
    }
  }

  fn transition_turn_to_ending(&mut self, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Transitioning current turn to ending");
    self.set_turn_state(TurnState::Ending, ctx);
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
      },
    );
  }

  fn advance_turn(&mut self, ctx: &ServiceContext) {
    let max_turn_count = ctx.config.get(&configs::TURN_COUNT);
    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!(target: "frontbox::game_manager", "Game state should have been initialized when starting the game");
      return;
    };

    ctx.deactivate_system_group(&self.player_groups[game_state.current_player() as usize]);
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

    self.start_turn(ctx);
  }

  /// Play is over. The game stays started until `GameEnding` completes.
  fn start_game_ending(&mut self, reason: GameEndReason, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Game ending ({:?})", reason);
    self.game_ending = true;
    self.draining = false;
    self.pending_drains.clear();
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
  fn end_game(&mut self, scores: Vec<(&'static str, u32)>, ctx: &ServiceContext) {
    log::info!(target: "frontbox::game_manager", "Game ended");
    for group in self.player_groups.drain(..) {
      ctx.despawn_system_group(group);
    }
    self.game_state = None;
    self.game_ending = false;
    self.set_turn_state(TurnState::Idle, ctx);
    ctx.for_system(self.handle).emit(GameEnded { scores });
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
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if self.game_ending {
      // turn routines still running after a quit complete on their own, and are ignored
      if let Some(done) = event.downcast_ref::<RoutineCompleted<GameEnding>>() {
        self.end_game(done.routine.scores.clone(), ctx.into());
      }
      return;
    }

    match self.turn_state {
      TurnState::Idle => {}
      TurnState::Starting if !self.ball_served => {
        if event.is::<RoutineCompleted<TurnStarting>>() {
          self.serve_ball(ctx.into());
        }
      }
      TurnState::Starting => {
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
      TurnState::Active => {
        if let Some(BallsExitedPlay { count, in_play }) = event.downcast_ref::<BallsExitedPlay>() {
          self.drained(*count, *in_play, ctx.into());
        } else if let Some(done) = event.downcast_ref::<RoutineCompleted<BallDraining>>() {
          self.drain_decided(done, ctx.into());
        }
      }
      TurnState::Ending => {
        if event.is::<RoutineCompleted<TurnEnding>>() {
          self.advance_turn(ctx.into());
        }
      }
    }
  }
}

impl GameManagement for CompetitiveGame {
  fn add_player(&mut self, ctx: &ServiceContext) {
    let mut game_started_just_now = false;
    if !self.is_game_started() {
      self.start_game(ctx);
      game_started_just_now = true;
    }

    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!(target: "frontbox::game_manager", "Game state should have been initialized when starting the game");
      return;
    };

    if game_state.player_count() >= game_state.max_players() {
      log::warn!(
        target: "frontbox::game_manager",
        "Max players reached ({}), cannot add more players",
        game_state.max_players()
      );
      return;
    }

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

    if game_started_just_now {
      self.start_turn(ctx);
    }
  }

  fn quit_game(&mut self, ctx: &ServiceContext) {
    let Some(player) = self.game_state.as_ref().map(GameState::current_player) else {
      return;
    };
    if self.game_ending {
      log::info!(target: "frontbox::game_manager", "Quit while the game is already ending, ignored");
      return;
    }

    log::info!(target: "frontbox::game_manager", "Quitting game");
    ctx.metrics.inc(&metrics::GAMES_QUIT);
    // the turn is over without a TurnEnding; its group's systems stop as it deactivates
    self.set_turn_state(TurnState::Ending, ctx);
    let ctx = &ctx.for_system(self.handle);
    ctx.deactivate_system_group(&self.player_groups[player as usize]);
    ctx.metrics.flush();
    self.start_game_ending(GameEndReason::Quit, ctx.into());
  }

  fn is_player_addable(&self) -> bool {
    if self.game_ending {
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

  fn is_game_started(&self) -> bool {
    self.game_state.is_some()
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
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ball_management::LaunchMethod;

  fn done<E: Event>(routine: E, completion: RoutineCompletion) -> RoutineCompleted<E> {
    RoutineCompleted {
      routine,
      completion,
    }
  }

  struct Fixture {
    game: CompetitiveGame,
    ctx: TestContext,
  }

  impl Fixture {
    /// A one-player game of `turns` turns, with the first turn's ball in play
    fn playing(turns: u8) -> Self {
      let ctx = TestContext::default();
      ctx.svc_ctx().config.set(&configs::TURN_COUNT, turns);
      let mut game = CompetitiveGame::new(1, Vec::new(), SwitchQ::names([]));
      game.on_spawn(&ctx.sys_ctx());
      game.add_player(&ctx.svc_ctx());
      let mut f = Self { game, ctx };
      f.event(done(
        TurnStarting {
          current_player: 0,
          turn: 0,
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
      self.event(done(
        TurnEnding {
          current_player,
          turn,
        },
        RoutineCompletion::AllSteps,
      ));
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
  fn the_game_stays_started_until_game_ending_completes() {
    let mut f = Fixture::playing(1);
    f.play_out_turn();
    let ending = f.game_endings().remove(0);
    assert!(f.game.is_game_started());
    assert!(!f.game.is_player_addable(), "no new game during GameEnding");

    f.event(done(ending, RoutineCompletion::AllSteps));
    assert!(!f.game.is_game_started());
    let ended: Vec<_> = f
      .ctx
      .events_emitted()
      .iter()
      .filter_map(|e| {
        e.event
          .downcast_ref::<GameEnded>()
          .map(|e| e.scores.clone())
      })
      .collect();
    assert_eq!(ended, vec![vec![("player1", 0)]]);
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
    f.game.quit_game(&f.ctx.svc_ctx());

    let endings = f.game_endings();
    assert_eq!(endings.len(), 1);
    assert_eq!(endings[0].reason, GameEndReason::Quit);
    assert_eq!(f.game.turn_state, TurnState::Ending);
  }

  #[test]
  fn the_turn_is_ignored_once_the_game_is_ending() {
    let mut f = Fixture::playing(3);
    f.game.quit_game(&f.ctx.svc_ctx());
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

    f.game.quit_game(&f.ctx.svc_ctx());
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
    assert_eq!(f.game.turn_state, TurnState::Ending);
  }
}
