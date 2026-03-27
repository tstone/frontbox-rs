use frontbox::plugins::{Trough, TroughFull};
use frontbox::prelude::*;

use crate::*;

const PLAYER_GROUP_NAMES: [&str; 6] = [
  "player1", "player2", "player3", "player4", "player5", "player6",
];

pub mod operator_config {
  use frontbox::prelude::ConfigItem;

  pub const TURN_COUNT: ConfigItem = ConfigItem::Integer {
    value: 3,
    default: 3,
    min: Some(1),
    max: Some(5),
    name: "Turn Count",
    description: "The current turn count for the player. This is automatically incremented at the end of each turn.",
    units: None,
  };
}

pub struct CompetitiveGame {
  /// This is the template to spin up a new group for the player
  systems_template: Vec<ChildSystemContainer>,
  max_players: u8,
  ball_in_play_switches: HardwareSelection,
  game_state: Option<GameState>,
}

impl CompetitiveGame {
  pub fn new(
    max_players: u8,
    player_template: Vec<ChildSystemContainer>,
    ball_in_play_switches: HardwareSelection,
  ) -> Self {
    Self {
      systems_template: player_template,
      max_players,
      ball_in_play_switches,
      game_state: None,
    }
  }

  fn start_game(&mut self, ctx: &Context) {
    log::info!("Starting game with max players: {}", self.max_players);
    self.game_state = Some(GameState::competitive(self.max_players));
    ctx.emit(GameStarted);
  }

  fn start_turn(&mut self, ctx: &Context, systems: &Systems) {
    let game_state = self.game_state.as_mut().unwrap();
    game_state.set_current_player_turn_state(TurnState::Beginning);
    ctx.emit(PlayerTurnBeginning::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));

    if let Some(trough) = systems.get::<Trough>() {
      trough.eject(ctx, systems);
    }
  }

  fn transition_turn_to_active(&mut self, ctx: &Context) {
    log::debug!("Transitioning current turn to active");
    let game_state = self.game_state.as_mut().unwrap();
    game_state.set_current_player_turn_state(TurnState::Active);
    ctx.emit(PlayerTurnActive::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn transition_turn_to_ending(&mut self, ctx: &Context) {
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
  fn on_shutdown(&mut self, ctx: &Context, _systems: &Systems) {
    if let Some(game_state) = &self.game_state {
      for player in 0..game_state.player_count() {
        let group_name = PLAYER_GROUP_NAMES[player as usize];
        ctx.despawn_system_group(group_name);
      }
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, _systems: &Systems) {
    if let Some(game_state) = &mut self.game_state {
      match game_state.current_player_turn_state() {
        TurnState::Beginning => {
          if let Some(e) = event.downcast_ref::<SwitchClosed>() {
            if self.ball_in_play_switches.matches_switch(&e.switch) {
              self.transition_turn_to_active(ctx);
            }
          }
        }
        TurnState::Active => {
          if let Some(_) = event.downcast_ref::<TroughFull>() {
            self.transition_turn_to_ending(ctx);
          }
        }
        _ => {}
      }
    }
  }
}

impl GameManagement for CompetitiveGame {
  fn add_player(&mut self, ctx: &Context, systems: &Systems) {
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
    let copy = self
      .systems_template
      .iter()
      .map(|system| system.clone())
      .collect::<Vec<_>>();

    let group_name = PLAYER_GROUP_NAMES[game_state.player_count() as usize];
    ctx.spawn_system_group(group_name, copy, false);

    if game_started_just_now {
      ctx.emit(GameStarted);
    }

    ctx.emit(PlayerAdded);

    if game_started_just_now {
      self.start_turn(ctx, systems);
    }
  }

  fn advance_turn(&mut self, ctx: &Context, systems: &Systems) {
    let max_turn_count = systems
      .expect::<OperatorConfig>()
      .get_integer("turn_count")
      .unwrap_or(3);

    let game_state = if let Some(game_state) = &mut self.game_state {
      game_state
    } else {
      log::error!("Game state should have been initialized when starting the game");
      return;
    };

    // increment the previous player's turn before advancing to next player/turn
    game_state.increment_current_player_turn();
    game_state.advance_turn();

    // Verify we haven't gone over the turn limit
    if game_state.current_player_turn() >= max_turn_count as u8 {
      self.end_game(ctx);
      return;
    }

    log::info!(
      "Advancing turn (player: {}, turn: {})",
      game_state.current_player(),
      game_state.current_player_turn()
    );

    self.start_turn(ctx, systems);
  }

  fn end_game(&mut self, ctx: &Context) {
    log::info!("Ending game");

    // verify the game is already running
    if !self.is_game_started() {
      return;
    }

    self.game_state = None;
    ctx.emit(GameEnded);
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

  fn clear_multiplier(&mut self) {
    if let Some(game_state) = &mut self.game_state {
      if let Some(multiplier) = game_state.current_player_multiplier_mut() {
        *multiplier = 1.0;
      }
    }
  }

  fn set_multiplier(&mut self, multiplier: f32) {
    if let Some(game_state) = &mut self.game_state {
      if let Some(current_multiplier) = game_state.current_player_multiplier_mut() {
        *current_multiplier = multiplier;
      }
    }
  }

  fn add_points(&mut self, points: u32, ctx: &Context) {
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
