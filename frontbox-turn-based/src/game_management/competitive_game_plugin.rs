use frontbox::prelude::*;
use frontbox::tags::{Playfield, StartButton};

use crate::{CompetitiveGame, FreePlay, GameManager};

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

pub struct CompetitiveGamePlugin {
  max_players: u8,
  systems_template: Vec<ChildSystemContainer>,
  start_button_switch: HardwareSelection,
  ball_in_play_switches: HardwareSelection,
}

impl CompetitiveGamePlugin {
  pub fn config() -> &'static CompetitiveGamePluginConfig {
    &CONFIG
  }

  pub fn new(systems_template: Vec<ChildSystemContainer>) -> Self {
    Self {
      systems_template,
      max_players: 4,
      start_button_switch: HardwareSelection::tag::<StartButton>(),
      ball_in_play_switches: HardwareSelection::tag::<Playfield>(),
    }
  }

  pub fn max_players(mut self, max_players: u8) -> Self {
    self.max_players = max_players;
    self
  }

  pub fn systems(mut self, systems: Vec<ChildSystemContainer>) -> Self {
    self.systems_template.extend(systems);
    self
  }

  pub fn ball_in_play_switches(mut self, switches: HardwareSelection) -> Self {
    self.ball_in_play_switches = switches;
    self
  }

  pub fn start_button_switch(mut self, switch: HardwareSelection) -> Self {
    self.start_button_switch = switch;
    self
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

    app.system(GameManager::new(CompetitiveGame::new(
      self.max_players,
      self.systems_template.clone(),
      self.ball_in_play_switches.clone(),
    )));

    // TODO: need some kind of freeplay/credits switching system
    app.system(FreePlay::new(self.start_button_switch.clone()));
  }
}
