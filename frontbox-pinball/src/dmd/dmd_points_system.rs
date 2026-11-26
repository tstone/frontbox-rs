use frontbox_core::prelude::*;
use frontbox_display::prelude::*;

use crate::dmd::open_scene;
use crate::{GameManagerExt, GamePhase, GameSession};

const POINTS_SCENE: &str = "frontbox/128x32/points.scene";
const TRANSITION_SCENE: &str = "frontbox/128x32/player_transition.scene";
/// Players the points scene has a score for
const PLAYERS: u8 = 4;

/// Shows the game's points on the DMD while a game is running: `frontbox/128x32/points.scene`, with each player's score,
/// the current player, and their ball. When the turn passes to another player, `frontbox/128x32/player_transition.scene`
/// plays once beneath it, centered on that player's score. A game replaces either by putting its own file under the same
/// name in its content folder.
///
/// ```rust,ignore
/// app.system(DmdPointsSystem::new());
/// ```
pub struct DmdPointsSystem {
  z: i8,
  points: Option<ScenePlayer>,
  transition: Option<ScenePlayer>,
  /// The variables last sent, so they're only sent again when they change
  sent: Vec<(String, Value)>,
  /// The current player when the points were last sent, from 0. `None` before a game's first.
  player: Option<u8>,
}

impl DmdPointsSystem {
  pub fn new() -> Self {
    Self {
      z: 0,
      points: None,
      transition: None,
      sent: Vec::new(),
      player: None,
    }
  }

  /// Draws the points at `z` among everything on the display, and the transition just beneath them. 0 unless this says
  /// otherwise.
  pub fn z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

impl Default for DmdPointsSystem {
  fn default() -> Self {
    Self::new()
  }
}

impl System for DmdPointsSystem {
  fn description() -> Option<&'static str> {
    Some("Shows each player's score and ball on the DMD during a game")
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    !ctx.states.is(GamePhase::None)
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    let open = |name, sends, z| {
      open_scene(&ctx.resources, name, sends, z)
        .map(|scene| ScenePlayer::new(PlayableScene::new(scene)))
    };
    self.points = open(POINTS_SCENE, POINTS_SENDS, self.z);
    self.transition = open(TRANSITION_SCENE, TRANSITION_SENDS, self.z.saturating_sub(1));
  }

  /// A new game starts with no transition, and its points all sent afresh
  fn on_reactivate(&mut self, _ctx: &SystemContext) {
    self.sent.clear();
    self.player = None;
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    let read = ctx.game_manager().and_then(|game| {
      let state = game.session()?;
      Some((state.current_player(), points_variables(state)))
    });
    let Some((player, variables)) = read else {
      return;
    };

    if self.player.is_some_and(|last| last != player)
      && let Some(transition) = &mut self.transition
    {
      transition.set_variable(
        "center",
        Value::named(format!("player_{}", player + 1)),
        ctx,
      );
      transition.restart(ctx);
    }
    self.player = Some(player);

    let Some(points) = &mut self.points else {
      return;
    };
    if variables != self.sent {
      for (name, value) in &variables {
        points.set_variable(name.clone(), value.clone(), ctx);
      }
      self.sent = variables;
    }
    points.start(ctx);
  }
}

/// What the points scene is sent for `state`. The scores along the bottom are only shown with more than one player.
fn points_variables(state: &GameSession) -> Vec<(String, Value)> {
  let (count, current) = (state.player_count(), state.current_player());
  let score = |player| state.player_score(player).copied().unwrap_or(0) as i64;
  let mut variables = vec![
    ("score".to_string(), Value::Int(score(current))),
    ("player".to_string(), Value::Int(current as i64 + 1)),
    (
      "ball".to_string(),
      Value::Int(state.current_player_turn() as i64 + 1),
    ),
  ];
  for player in 0..PLAYERS {
    let n = player + 1;
    let color = if player == current {
      "current"
    } else {
      "waiting"
    };
    variables.push((format!("score_{n}"), Value::Int(score(player))));
    variables.push((format!("player_{n}_color"), Value::named(color)));
    variables.push((
      format!("player_{n}_visible"),
      Value::Bool(count > 1 && player < count),
    ));
  }
  variables
}

const POINTS_SENDS: Sends = Sends {
  variables: &[
    SentVariable {
      name: "score",
      sent_as: VariableType::Number,
      values: &[],
      about: "The current player's score",
    },
    SentVariable {
      name: "player",
      sent_as: VariableType::Number,
      values: &[],
      about: "The current player, from 1",
    },
    SentVariable {
      name: "ball",
      sent_as: VariableType::Number,
      values: &[],
      about: "The current player's ball, from 1",
    },
    SentVariable {
      name: "score_1",
      sent_as: VariableType::Number,
      values: &[],
      about: "Player 1's score",
    },
    SentVariable {
      name: "score_2",
      sent_as: VariableType::Number,
      values: &[],
      about: "Player 2's score",
    },
    SentVariable {
      name: "score_3",
      sent_as: VariableType::Number,
      values: &[],
      about: "Player 3's score",
    },
    SentVariable {
      name: "score_4",
      sent_as: VariableType::Number,
      values: &[],
      about: "Player 4's score",
    },
    SentVariable {
      name: "player_1_color",
      sent_as: VariableType::Color,
      values: &["current", "waiting"],
      about: "Player 1's score's color",
    },
    SentVariable {
      name: "player_2_color",
      sent_as: VariableType::Color,
      values: &["current", "waiting"],
      about: "Player 2's score's color",
    },
    SentVariable {
      name: "player_3_color",
      sent_as: VariableType::Color,
      values: &["current", "waiting"],
      about: "Player 3's score's color",
    },
    SentVariable {
      name: "player_4_color",
      sent_as: VariableType::Color,
      values: &["current", "waiting"],
      about: "Player 4's score's color",
    },
    SentVariable {
      name: "player_1_visible",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether player 1's score is shown",
    },
    SentVariable {
      name: "player_2_visible",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether player 2's score is shown",
    },
    SentVariable {
      name: "player_3_visible",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether player 3's score is shown",
    },
    SentVariable {
      name: "player_4_visible",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether player 4's score is shown",
    },
  ],
};

const TRANSITION_SENDS: Sends = Sends {
  variables: &[SentVariable {
    name: "center",
    sent_as: VariableType::Vec2,
    values: &["player_1", "player_2", "player_3", "player_4"],
    about: "Where the player whose turn it is now has their score",
  }],
};

#[cfg(test)]
mod tests {
  use super::*;

  fn variable<'a>(variables: &'a [(String, Value)], name: &str) -> &'a Value {
    &variables
      .iter()
      .find(|(variable, _)| variable == name)
      .unwrap()
      .1
  }

  fn game(players: u8) -> GameSession {
    let mut state = GameSession::competitive(4);
    (0..players).for_each(|_| state.increment_player_count());
    state
  }

  #[test]
  fn the_current_player_is_counted_from_1_and_their_score_is_in_current() {
    let mut state = game(2);
    state.advance_turn();
    let variables = points_variables(&state);
    assert_eq!(variable(&variables, "player"), &Value::Int(2));
    assert_eq!(variable(&variables, "ball"), &Value::Int(1));
    assert_eq!(
      variable(&variables, "player_2_color"),
      &Value::named("current")
    );
    assert_eq!(
      variable(&variables, "player_1_color"),
      &Value::named("waiting")
    );
  }

  #[test]
  fn the_scores_along_the_bottom_are_for_the_players_in_a_game_of_more_than_one() {
    let alone = points_variables(&game(1));
    assert_eq!(variable(&alone, "player_1_visible"), &Value::Bool(false));
    let two = points_variables(&game(2));
    assert_eq!(variable(&two, "player_2_visible"), &Value::Bool(true));
    assert_eq!(variable(&two, "player_3_visible"), &Value::Bool(false));
  }

  #[test]
  fn the_bundled_scenes_fit_what_is_sent() {
    let resources = crate::dmd::bundle();
    for (name, sends) in [
      (POINTS_SCENE, POINTS_SENDS),
      (TRANSITION_SCENE, TRANSITION_SENDS),
    ] {
      assert_eq!(
        sends.check(&Scene::open(&resources, name).unwrap()),
        [],
        "{name}"
      );
    }
  }
}
