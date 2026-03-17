use frontbox::prebuilt::TroughFull;
use frontbox::prelude::*;

use crate::*;

const PLAYER_GROUP_NAMES: [&str; 6] = [
  "player1", "player2", "player3", "player4", "player5", "player6",
];

pub mod operator_config {
  use frontbox::prelude::ConfigItem;

  pub const TURN_COUNT: ConfigItem = ConfigItem::Integer {
    current: 3,
    default: 3,
    min: 1,
    max: 5,
    name: "Turn Count",
    description: "The current turn count for the player. This is automatically incremented at the end of each turn.",
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
/// ## Operator Config
///
///
/// # Arguments:
/// - `max_players` - The maximum number of players allowed in a game
/// - `ball_in_play_switches` - A list of switches that can be used to detect when the ball becomes in play. This could be a plunge lane exit switch, or a list of playfield switches.
pub struct IndividualPlayerSystem {
  /// This is the template to spin up a new group for the player
  systems_template: Vec<Box<dyn ChildSystem>>,
  max_players: u8,
  ball_in_play_switches: Vec<&'static str>,
}

impl IndividualPlayerSystem {
  ///
  pub fn new(
    max_players: u8,
    ball_in_play_switches: Vec<&'static str>,
    initial_scene: Vec<Box<dyn ChildSystem>>,
  ) -> Box<Self> {
    Box::new(Self {
      systems_template: initial_scene,
      max_players,
      ball_in_play_switches,
    })
  }

  fn is_game_started(&self, ctx: &Context) -> bool {
    ctx.get::<GameState>().is_some()
  }

  fn start_game(&self, ctx: &mut Context) {
    ctx.insert(GameState::new(self.max_players));
    ctx.insert(GameStartState::PlayerAddable);
    ctx.emit(GameStarted);
  }

  fn add_player(&mut self, ctx: &mut Context) {
    let mut game_started = false;
    if !self.is_game_started(ctx) {
      self.start_game(ctx);
      game_started = true;
    }

    let game_state = ctx.expect_mut::<GameState>();
    game_state.player_count += 1;

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
      ctx.insert(CurrentPlayerTurnState::Beginning);
      ctx.emit(PlayerTurnBeginning::new(0, 0));
    }
  }

  fn transition_turn_to_active(&self, ctx: &mut Context) {
    ctx.insert(CurrentPlayerTurnState::Active);
    let game_state = ctx.expect::<GameState>();
    ctx.emit(PlayerTurnActive::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn transition_turn_to_ending(&self, ctx: &mut Context) {
    ctx.insert(CurrentPlayerTurnState::Ending);
    let game_state = ctx.expect::<GameState>();
    ctx.emit(PlayerTurnEnding::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
  }

  fn advance_turn(&self, ctx: &mut Context) {
    let mut game_state = ctx.cloned::<GameState>().unwrap();

    let mut next_player = game_state.current_player + 1;
    if next_player >= game_state.player_count {
      next_player = 0;
    }
    game_state.current_player = next_player;
    game_state.player_turns[game_state.current_player as usize] += 1;

    // Verify we haven't gone over the turn limit
    let max_turn_count = ctx
      .expect::<OperatorConfig>()
      .get_value_as_integer(operator_config::TURN_COUNT.name())
      .unwrap_or(3);
    if game_state.player_turns[game_state.current_player as usize] > max_turn_count as u8 {
      self.end_game(ctx);
      return;
    }

    ctx.insert(CurrentPlayerTurnState::Beginning);
    ctx.emit(PlayerTurnBeginning::new(
      game_state.current_player(),
      game_state.current_player_turn(),
    ));
    ctx.insert(game_state);
  }

  fn end_game(&self, ctx: &mut Context) {
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
  fn on_command(&mut self, command: &dyn Command, _caller_id: u64, ctx: &mut Context) {
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

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if self.is_game_started(ctx) {
      match ctx.get::<CurrentPlayerTurnState>() {
        Some(CurrentPlayerTurnState::Beginning) => {
          if let Some(e) = event.downcast_ref::<SwitchClosed>() {
            if self.ball_in_play_switches.contains(&e.switch.name) {
              self.transition_turn_to_active(ctx);
            }
          }
        }
        None => {
          if let Some(_) = event.downcast_ref::<TroughFull>() {
            self.transition_turn_to_ending(ctx);
          }
        }
        _ => {}
      }
    }
  }
}
