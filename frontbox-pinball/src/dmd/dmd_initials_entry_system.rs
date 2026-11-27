use frontbox_core::prelude::*;
use frontbox_display::prelude::*;

use crate::dmd::open_scene;
use crate::{InitialsEntry, LeaderboardsSystem};

const INITIALS_ENTRY_SCENE: &str = "frontbox/128x32/initials_entry.scene";
/// How long each thing the player made is named, when they made more than one
const TITLE_TIME: Duration = Duration::from_secs(2);

/// Shows initials being entered on the DMD: `frontbox/128x32/initials_entry.scene`, with the player, what they made
/// (each in turn, when it's more than one), and their initials so far. It only draws: `LeaderboardsSystem` takes the
/// initials, and this shows whichever it's taking. A game replaces the scene by putting its own file under the same
/// name in its content folder.
///
/// ```rust,ignore
/// app.system(DmdInitialsEntrySystem::new().z(10));
/// ```
pub struct DmdInitialsEntrySystem {
  z: i8,
  scene: Option<ScenePlayer>,
  /// The variables last sent, so they're only sent again when they change
  sent: Vec<(String, Value)>,
  /// Whose initials are shown, from 0, and for how long they have been
  showing: Option<(u8, Duration)>,
}

impl DmdInitialsEntrySystem {
  pub fn new() -> Self {
    Self {
      z: 0,
      scene: None,
      sent: Vec::new(),
      showing: None,
    }
  }

  /// Draws the entry at `z` among everything on the display, which wants to be above the points. 0 unless this says
  /// otherwise.
  pub fn z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

impl Default for DmdInitialsEntrySystem {
  fn default() -> Self {
    Self::new()
  }
}

impl System for DmdInitialsEntrySystem {
  fn description() -> Option<&'static str> {
    Some("Shows a player's initials on the DMD as they enter them for the leaderboards")
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.scene = open_scene(&ctx.resources, INITIALS_ENTRY_SCENE, ENTRY_SENDS, self.z)
      .map(|scene| ScenePlayer::new(PlayableScene::new(scene)));
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    let Some(scene) = &mut self.scene else {
      return;
    };
    let entry = ctx
      .systems
      .get::<LeaderboardsSystem>()
      .and_then(|leaderboards| leaderboards.entry().cloned());
    let Some(entry) = entry else {
      if self.showing.take().is_some() {
        scene.stop(ctx);
        self.sent.clear();
      }
      return;
    };

    let shown_for = match self.showing {
      Some((player, shown_for)) if player == entry.player() => shown_for + delta,
      _ => Duration::ZERO,
    };
    self.showing = Some((entry.player(), shown_for));

    let variables = entry_variables(&entry, shown_for);
    if variables != self.sent {
      for (name, value) in &variables {
        scene.set_variable(name.clone(), value.clone(), ctx);
      }
      self.sent = variables;
    }
    scene.start(ctx);
  }
}

/// What the entry scene is sent for `entry`, which has been showing for `shown_for`
fn entry_variables(entry: &InitialsEntry, shown_for: Duration) -> Vec<(String, Value)> {
  let placements = entry.placements();
  let title = if placements.is_empty() {
    String::new()
  } else {
    let at = (shown_for.as_millis() / TITLE_TIME.as_millis()) as usize % placements.len();
    placements[at].board.place_title(placements[at].rank)
  };

  let mut variables = vec![
    (
      "player".to_string(),
      Value::from(format!("Player {}", entry.player() + 1)),
    ),
    ("title".to_string(), Value::from(title)),
  ];
  for (at, letter) in entry.letters().into_iter().enumerate() {
    let n = at + 1;
    let letter = letter.map(String::from).unwrap_or_default();
    variables.push((format!("letter_{n}"), Value::from(letter)));
    variables.push((
      format!("letter_{n}_current"),
      Value::Bool(at == entry.cursor()),
    ));
  }
  variables
}

const ENTRY_SENDS: Sends = Sends {
  variables: &[
    SentVariable {
      name: "player",
      sent_as: VariableType::Text,
      values: &[],
      about: "Whose initials these are, e.g. Player 1",
    },
    SentVariable {
      name: "title",
      sent_as: VariableType::Text,
      values: &[],
      about: "What the player made, e.g. Grand Champion. Each in turn, when it's more than one",
    },
    SentVariable {
      name: "letter_1",
      sent_as: VariableType::Text,
      values: &[],
      about: "The first initial. `<` when the player is choosing to go back",
    },
    SentVariable {
      name: "letter_2",
      sent_as: VariableType::Text,
      values: &[],
      about: "The second initial, empty until it's reached",
    },
    SentVariable {
      name: "letter_3",
      sent_as: VariableType::Text,
      values: &[],
      about: "The third initial, empty until it's reached",
    },
    SentVariable {
      name: "letter_1_current",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether the first initial is the one being chosen",
    },
    SentVariable {
      name: "letter_2_current",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether the second initial is the one being chosen",
    },
    SentVariable {
      name: "letter_3_current",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether the third initial is the one being chosen",
    },
  ],
};

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use frontbox_scene::SceneSystem;

  use super::*;
  use crate::{GameEndReason, GameEnding, Leaderboard};

  static SCORES: LazyLock<Leaderboard<u64>> = LazyLock::new(|| {
    Leaderboard::highest("High Scores", 2)
      .key("entry_scores")
      .rank_titles(["Grand Champion"])
  });
  static LOOPS: LazyLock<Leaderboard<u32>> =
    LazyLock::new(|| Leaderboard::highest("Loop Champion", 1).key("entry_loops"));

  fn text<'a>(variables: &'a [(String, Value)], name: &str) -> &'a Value {
    &variables.iter().find(|(n, _)| n == name).unwrap().1
  }

  #[test]
  fn the_bundled_scene_fits_what_is_sent() {
    let resources = crate::dmd::bundle();
    assert_eq!(
      ENTRY_SENDS.check(&Scene::open(&resources, INITIALS_ENTRY_SCENE).unwrap()),
      []
    );
  }

  #[test]
  fn the_entry_is_shown_while_initials_are_being_taken() {
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context.set_resources(crate::dmd::bundle());
    let path = std::env::temp_dir().join(format!("frontbox-dmd-entry-{}.toml", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut leaderboards = LeaderboardsSystem::new(&path)
      .final_scores(&SCORES)
      .board(&LOOPS)
      .initials_entry(
        SwitchQ::name("left"),
        SwitchQ::name("right"),
        SwitchQ::name("start"),
      );
    leaderboards.submit(&LOOPS, 0, 5);
    let game = GameEnding {
      scores: vec![100],
      reason: GameEndReason::Completed,
    };
    leaderboards.on_routine(&game, &context.sys_ctx());
    let entry = leaderboards.entry().unwrap().clone();
    context.insert_system(leaderboards);

    let mut dmd = DmdInitialsEntrySystem::new();
    dmd.on_spawn(&context.sys_ctx());
    dmd.on_tick(Duration::ZERO, &context.sys_ctx());
    assert!(dmd.scene.as_ref().unwrap().is_started());
    assert_eq!(*text(&dmd.sent, "player"), Value::from("Player 1"));
    assert_eq!(*text(&dmd.sent, "letter_1"), Value::from("A"));
    assert_eq!(*text(&dmd.sent, "letter_1_current"), Value::Bool(true));
    assert_eq!(*text(&dmd.sent, "letter_2"), Value::from(""));

    // what they made is named one after another
    let title = |after| text(&entry_variables(&entry, after), "title").clone();
    assert_eq!(title(Duration::ZERO), Value::from("Grand Champion"));
    assert_eq!(title(TITLE_TIME), Value::from("Loop Champion"));
    assert_eq!(title(TITLE_TIME * 2), Value::from("Grand Champion"));
  }
}
