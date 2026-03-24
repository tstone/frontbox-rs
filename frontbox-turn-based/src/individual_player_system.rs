use frontbox::plugins::{TroughEject, TroughFull};
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

/// This system provides two main benefits:
///
///   1. Systems are organized by player, so that each player can have their own set of systems that are active only during their turn.
///   2. Player turn management
///
/// ## Outputs
/// - Event: `PlayerTurnBeginning` - Emitted at the start of a player's turn, but before the ball is in play (launched).
/// - Event: `PlayerTurnActive` - Emitted when the ball becomes in play.
/// - Event: `PlayerTurnEnding` - Emitted when the ball goes out of play and is in the trough.
/// - Command: `TroughEject` - Fired at the start of a player's turn
///
/// ## Inputs
/// - Command: `AddPlayer` - Adds a player to the game
/// - Command: `AdvanceTurn` - Advances the turn to the next player. Only processed after a `PlayerTurnEnding` event has been emitted.
/// - Command: `EndGame` - Ends the current game (only valid if a game is started).
/// - Event: `TroughFull` - Used to detect when the ball has gone out of play.
///
/// ## Interrupts
/// - `TroughFull` - Interrupting this event will prevent the player turn from ending. This can be used to implement mechanics like ball saves or extra balls.
///
/// # Arguments:
/// - `max_players` - The maximum number of players allowed in a game
/// - `ball_in_play_switches` - A list of switches that can be used to detect when the ball becomes in play. This could be a plunge lane exit switch, or a list of playfield switches.
pub struct IndividualPlayerSystem {
  /// This is the template to spin up a new group for the player
  systems_template: Vec<Box<dyn ChildSystem>>,
  max_players: u8,
  ball_in_play_switch_group: &'static str,
}

impl IndividualPlayerSystem {
  pub fn new(
    max_players: u8,
    ball_in_play_switch_group: &'static str,
    initial_scene: Vec<Box<dyn ChildSystem>>,
  ) -> Box<Self> {
    Box::new(Self {
      systems_template: initial_scene,
      max_players,
      ball_in_play_switch_group,
    })
  }

  fn start_game(&self, ctx: &mut Context) {
    log::info!("Starting game with max players: {}", self.max_players);
    ctx.insert(GameState::new(self.max_players));
    ctx.insert(GameStartState::PlayerAddable);
    ctx.emit(GameStarted);
  }

  fn add_player(&mut self, ctx: &mut Context) {
    let mut game_started = false;
    if !ctx.is_game_started() {
      self.start_game(ctx);
      game_started = true;
    }

    if let Some(game_state) = ctx.get::<GameState>() {
      if game_state.player_count >= game_state.max_players {
        log::warn!(
          "Max players reached ({}), cannot add more players",
          game_state.max_players
        );
        return;
      }
    }

    let game_state = ctx.expect_mut::<GameState>();
    game_state.player_count += 1;
    log::info!(
      "Adding player to game (current count: {})",
      game_state.player_count
    );

    // create copy of systems for new player as a new system group
    let copy = self
      .systems_template
      .iter()
      .map(|system| dyn_clone::clone_box(&**system))
      .collect::<Vec<_>>();

    let group_name = PLAYER_GROUP_NAMES[game_state.player_count as usize];
    ctx.spawn_system_group(group_name, copy, false);
    ctx.emit(PlayerAdded);

    if game_started {
      ctx.emit(GameStarted);
      self.start_turn(ctx);
    }
  }

  fn start_turn(&self, ctx: &mut Context) {
    ctx.insert(CurrentPlayerTurnState::Beginning);
    let game_state = ctx.expect::<GameState>();
    ctx.emit(PlayerTurnBeginning::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
    ctx.command(TroughEject);
  }

  fn transition_turn_to_active(&self, ctx: &mut Context) {
    log::debug!("Transitioning current turn to active");
    ctx.insert(CurrentPlayerTurnState::Active);
    let game_state = ctx.expect::<GameState>();
    ctx.emit(PlayerTurnActive::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn transition_turn_to_ending(&self, ctx: &mut Context) {
    log::debug!("Transitioning current turn to ending");
    ctx.insert(CurrentPlayerTurnState::Ending);
    let game_state = ctx.expect::<GameState>();
    ctx.emit(PlayerTurnEnding::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn advance_turn(&self, ctx: &mut Context) {
    let max_turn_count = ctx
      .expect::<OperatorConfig>()
      .get_value_as_integer(operator_config::TURN_COUNT.name())
      .unwrap_or(3);
    let game_state = ctx.expect_mut::<GameState>();
    let mut next_player = game_state.current_player + 1;
    if next_player >= game_state.player_count {
      next_player = 0;
    }
    game_state.current_player = next_player;
    game_state.player_turns[game_state.current_player as usize] += 1;

    // Verify we haven't gone over the turn limit
    if game_state.player_turns[game_state.current_player as usize] >= max_turn_count as u8 {
      self.end_game(ctx);
      return;
    }

    log::info!(
      "Advancing turn (player: {}, turn: {})",
      game_state.current_player,
      game_state.current_player_turn()
    );

    if next_player > 0 || game_state.player_turns[game_state.current_player as usize] > 0 {
      ctx.insert(GameStartState::NotStartable);
    }

    self.start_turn(ctx);
  }

  fn end_game(&self, ctx: &mut Context) {
    log::info!("Ending game");

    // verify the game is already running
    if ctx.get::<GameState>().is_none() {
      return;
    }

    ctx.remove::<GameState>();
    ctx.insert(GameStartState::GameStartable);
    ctx.emit(GameEnded);
  }
}

impl System for IndividualPlayerSystem {
  fn on_command(&mut self, command: &dyn Signal, _caller_id: u64, ctx: &mut Context) {
    if let Some(_) = command.downcast_ref::<AddPlayer>() {
      self.add_player(ctx);
    } else if let Some(_) = command.downcast_ref::<AdvanceTurn>() {
      self.advance_turn(ctx);
    }
  }

  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.insert(GameStartState::GameStartable);
    ctx.register_command::<AddPlayer>();
    ctx.register_command::<AdvanceTurn>();
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    if let Some(game_state) = ctx.get::<GameState>() {
      for player in 0..game_state.player_count {
        let group_name = PLAYER_GROUP_NAMES[player as usize];
        ctx.despawn_system_group(group_name);
      }
    }
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
    if ctx.is_game_started() {
      match ctx.get::<CurrentPlayerTurnState>() {
        Some(CurrentPlayerTurnState::Beginning) => {
          if let Some(e) = event.downcast_ref::<SwitchClosed>() {
            let ball_in_play_switches = ctx
              .expect::<SwitchGroups>()
              .get(&self.ball_in_play_switch_group)
              .cloned()
              .unwrap_or_default();

            if ball_in_play_switches.contains(&e.switch.name) {
              self.transition_turn_to_active(ctx);
            }
          }
        }
        Some(CurrentPlayerTurnState::Active) => {
          if let Some(_) = event.downcast_ref::<TroughFull>() {
            self.transition_turn_to_ending(ctx);
          }
        }
        _ => {}
      }
    }
  }
}
