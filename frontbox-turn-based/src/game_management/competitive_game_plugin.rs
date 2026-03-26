use frontbox::prelude::*;

use crate::{CompetitiveGame, FreePlay};

/// This plugin provides:
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
///
/// # Arguments:
/// - `max_players` - The maximum number of players allowed in a game
/// - `ball_in_play_switches` - A list of switches that can be used to detect when the ball becomes in play. This could be a plunge lane exit switch, or a list of playfield switches.
pub struct CompetitiveGamePlugin {
  systems_template: Vec<ChildSystemContainer>,
  max_players: u8,
  start_button_name: &'static str,
  ball_in_play_switches: Vec<&'static str>,
}

pub struct CompetitiveGamePluginConfig {
  pub turn_count: &'static str,
  pub payment_mode: &'static str,
  pub credits_required: &'static str,
}

const CONFIG: CompetitiveGamePluginConfig = CompetitiveGamePluginConfig {
  turn_count: "turn_count",
  payment_mode: "payment",
  credits_required: "credits_required",
};

impl CompetitiveGamePlugin {
  pub fn config() -> &'static CompetitiveGamePluginConfig {
    &CONFIG
  }

  pub fn new(
    max_players: u8,
    ball_in_play_switches: Vec<&'static str>,
    start_button_name: &'static str,
    systems_template: Vec<ChildSystemContainer>,
  ) -> Self {
    Self {
      systems_template,
      max_players,
      ball_in_play_switches,
      start_button_name,
    }
  }
}

impl Plugin for CompetitiveGamePlugin {
  fn build(&self, app: &mut App) {
    app.operator_config(
      OperatorConfigs::integer(CompetitiveGamePlugin::config().turn_count)
        .default(3)
        .min(1)
        .max(5)
        .name("Ball Count")
        .description("The maximum number of balls (turns) each player gets in a game"),
    );

    app.operator_config(
      OperatorConfigs::string(CompetitiveGamePlugin::config().payment_mode)
        .default("free_play".to_string())
        .options(vec!["free_play".to_string(), "coin_op".to_string()])
        .name("Payment Mode")
        .description("The payment mode for the game (e.g., Free Play, Coin Op)"),
    );

    app.operator_config(
      OperatorConfigs::integer(CompetitiveGamePlugin::config().credits_required)
        .default(4)
        .min(1)
        .max(32)
        .name("Credits Required")
        .description("The number of credits required to start the game"),
    );

    app.system(CompetitiveGame::new(
      self.max_players,
      self.ball_in_play_switches.clone(),
      self.systems_template.clone(),
    ));

    app.system(FreePlay::new(self.start_button_name));
  }
}
