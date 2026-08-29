use frontbox::prelude::*;
use frontbox::provided::{BallExitedPlungeLane, TroughFull, TroughSystem};

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
/// - Event: `PlayerTurnBeginning` - Emitted at the start of a player's turn, but before the ball is in play (launched).
/// - Event: `PlayerTurnActive` - Emitted when the ball becomes in play.
/// - Event: `PlayerTurnEnding` - Emitted when the ball goes out of play and is in the trough.
///
/// ## Interrupts
/// - `TroughFull` - Interrupting this event will prevent the player turn from ending. This can be used to implement mechanics like ball saves or extra balls.
pub struct CompetitiveGame {
  /// This is the template to spin up a new group for the player
  systems_template: Vec<ChildSystemContainer>,
  max_players: u8,
  ball_in_play_switches: SwitchQ,
  game_state: Option<GameState>,
  handle: SystemHandle,
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
      handle: SystemHandle::default(),
    }
  }

  fn player_group_name(player: u8) -> &'static str {
    PLAYER_GROUP_NAMES[player as usize]
  }

  fn start_game(&mut self, ctx: &ServiceContext) {
    log::info!("Starting game with max players: {}", self.max_players);
    self.game_state = Some(GameState::competitive(self.max_players));
    ctx.emit(GameStarted);
  }

  fn start_turn(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let game_state = self.game_state.as_mut().unwrap();

    ctx.activate_system_group(Self::player_group_name(game_state.current_player()));

    game_state.set_current_player_turn_state(TurnState::Beginning);
    ctx.emit(PlayerTurnBeginning::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));

    if let Some(trough) = ctx.get::<TroughSystem>() {
      trough.eject(ctx.into());
    }

    ctx.emit(Synchronize);
  }

  fn transition_turn_to_active(&mut self, ctx: &ServiceContext) {
    log::debug!("Transitioning current turn to active");
    let game_state = self.game_state.as_mut().unwrap();
    game_state.set_current_player_turn_state(TurnState::Active);
    ctx.emit(PlayerTurnActive::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn transition_turn_to_ending(&mut self, ctx: &ServiceContext) {
    log::debug!("Transitioning current turn to ending");
    let game_state = self.game_state.as_mut().unwrap();
    game_state.set_current_player_turn_state(TurnState::Ending);
    ctx.emit(PlayerTurnEnding::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }
}

impl System for CompetitiveGame {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
  }

  fn config_values(&self) -> Vec<&'static dyn GeneralizedConfigValue> {
    vec![&*configs::TURN_COUNT]
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    if let Some(game_state) = &self.game_state {
      for player in 0..game_state.player_count() {
        let group_name = Self::player_group_name(player);
        ctx.despawn_system_group(group_name);
      }
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(game_state) = &mut self.game_state {
      match game_state.current_player_turn_state() {
        TurnState::Beginning => {
          if event.is::<BallExitedPlungeLane>() {
            self.transition_turn_to_active(ctx.into());
          } else if let Some(e) = event.downcast_ref::<SwitchClosed>()
            && self.ball_in_play_switches.matches(&e.switch)
          {
            self.transition_turn_to_active(ctx.into());
          }
        }
        TurnState::Active if event.is::<TroughFull>() => {
          self.transition_turn_to_ending(ctx.into());
        }
        _ => {}
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
      log::error!("Game state should have been initialized when starting the game");
      return;
    };

    if game_state.player_count() >= game_state.max_players() {
      log::warn!(
        "Max players reached ({}), cannot add more players",
        game_state.max_players()
      );
      return;
    }

    game_state.increment_player_count();
    log::info!(
      "Adding player to game (current count: {})",
      game_state.player_count()
    );

    // create copy of systems for new player as a new system group
    let copy = self.systems_template.to_vec();

    let group_name = Self::player_group_name(game_state.player_count() - 1);
    ctx.spawn_system_group(group_name, copy, false);

    ctx.emit(PlayerAdded);

    if game_started_just_now {
      self.start_turn(ctx);
    }
  }

  fn advance_turn(&mut self, ctx: &ServiceContext) {
    let max_turn_count = ctx.operator_config.get(&configs::TURN_COUNT);
    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!("Game state should have been initialized when starting the game");
      return;
    };

    ctx.deactivate_system_group(Self::player_group_name(game_state.current_player()));

    // increment the previous player's turn before advancing to next player/turn
    game_state.increment_current_player_turn();
    game_state.advance_turn();

    // Verify we haven't gone over the turn limit
    if game_state.current_player_turn() >= max_turn_count {
      self.end_game(ctx);
      return;
    }

    log::info!(
      "Advancing turn (player: {}, turn: {})",
      game_state.current_player(),
      game_state.current_player_turn()
    );

    self.start_turn(ctx);
  }

  fn end_game(&mut self, ctx: &ServiceContext) {
    log::info!("Ending game");

    // verify the game is already running
    if !self.is_game_started() {
      return;
    }

    let game_state = self.game_state.as_ref().unwrap();
    let player_count = game_state.player_count();
    let scores: Vec<(&'static str, u32)> = match game_state {
      GameState::Competitive { player_scores, .. } => player_scores
        .iter()
        .take(player_count as usize)
        .enumerate()
        .map(|(idx, score)| (PLAYER_GROUP_NAMES[idx], *score))
        .collect(),
      _ => Vec::new(),
    };

    self.game_state = None;
    ctx.emit(GameEnded { scores });

    // Clear LEDs
    let ctx = ctx.for_system(self.handle);
    // TODO: this should actually invoke machine.clear_leds() or something, which emits an event for other systems to react to
    let machine = ctx.expect::<Machine>();
    for board in &ctx.exp_network {
      machine.set_all_leds(board.address, board.breakout, Rgba::black());
    }

    ctx.emit(Synchronize);
  }

  fn is_player_addable(&self) -> bool {
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

  fn turn_state(&self) -> Option<&TurnState> {
    self
      .game_state()
      .map(|state| state.current_player_turn_state())
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
        ctx.emit(PointsAdded::new(
          game_state.current_player(),
          points_received,
          total_points,
        ));
      }
    }
  }
}
