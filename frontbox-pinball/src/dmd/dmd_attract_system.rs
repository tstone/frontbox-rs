use frontbox_core::prelude::*;
use frontbox_display::content_with_bundled;
use frontbox_display::menu::DmdMenuSystem;
use frontbox_gfx::scene::{Value, VariableType};
use frontbox_scene::{PlayableScene, Scene, ScenePlayer, SentVariable, Sends};

use crate::dmd::open_scene;
use crate::{GameEnding, GamePhase, PlayerEntry};

const LAST_SCORE_SCENE: &str = "frontbox/128x32/attract/last_score.scene";
const PRESS_START_SCENE: &str = "frontbox/128x32/attract/press_start.scene";

/// Attract mode on the DMD: while no game is running and the operator menu isn't open, shows a list of screens one after
/// another, each for the same time, and starts over at the end of the list. The flipper buttons step back and forward
/// through them. When a game ends, it goes to the first of the last game's scores.
///
/// ```rust,ignore
/// app.system(
///   DmdAttractSystem::new(vec![
///     AttractScreen::Scene("dmd/title.scene"),
///     AttractScreen::LastScore(0),
///     AttractScreen::PressStart,
///     AttractScreen::LastScore(1),
///   ])
///   .flippers(cabinet::LEFT_FLIPPER.name, cabinet::RIGHT_FLIPPER.name),
/// );
/// ```
pub struct DmdAttractSystem {
  screens: Vec<AttractScreen>,
  /// Each screen's scene, opened when the system spawns. `None` for one that couldn't be, which is skipped.
  scenes: Vec<Option<Scene>>,
  /// The switches that step back and forward through the screens
  flippers: Option<(&'static str, &'static str)>,
  screen_duration: Duration,
  z: i8,
  /// The screen that's shown, or is shown next if none is
  current: usize,
  player: Option<ScenePlayer>,
  last_scores: Vec<(&'static str, u64)>,
  cue_id: Option<u64>,
}

impl DmdAttractSystem {
  pub fn new(screens: Vec<AttractScreen>) -> Self {
    Self {
      screens,
      scenes: Vec::new(),
      flippers: None,
      screen_duration: Duration::from_secs(6),
      z: 0,
      current: 0,
      player: None,
      last_scores: Vec::new(),
      cue_id: None,
    }
  }

  /// The switches that step back (`previous`) and forward (`next`) through the screens, such as the flipper buttons
  pub fn flippers(mut self, previous: &'static str, next: &'static str) -> Self {
    self.flippers = Some((previous, next));
    self
  }

  /// How long each screen is shown. 6 seconds unless this says otherwise.
  pub fn screen_duration(mut self, duration: Duration) -> Self {
    self.screen_duration = duration;
    self
  }

  /// Draws the screens at `z` among everything on the display. 0 unless this says otherwise.
  pub fn z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }

  /// Whether screen `index` has something to show now
  fn can_show(&self, index: usize, ctx: &SystemContext) -> bool {
    if self.scenes.get(index).is_none_or(Option::is_none) {
      return false;
    }
    match self.screens[index] {
      AttractScreen::Scene(_) => true,
      AttractScreen::LastScore(player) => player < self.last_scores.len(),
      AttractScreen::PressStart => ctx.states.is(PlayerEntry::Open),
    }
  }

  /// The first screen from `index` on, going by `step`, that has something to show. `None` if none does.
  fn showable_from(&self, index: usize, step: isize, ctx: &SystemContext) -> Option<usize> {
    let count = self.screens.len() as isize;
    (0..count).map(|n| (index as isize + n * step).rem_euclid(count) as usize).find(|&at| self.can_show(at, ctx))
  }

  /// Moves to the next screen that has something to show, `step` screens on (1, or -1 to go back), and shows it
  fn step(&mut self, step: isize, ctx: &SystemContext) {
    let count = self.screens.len() as isize;
    if count == 0 {
      return;
    }
    let from = (self.current as isize + step).rem_euclid(count) as usize;
    if let Some(next) = self.showable_from(from, step, ctx) {
      self.show(next, ctx);
    }
  }

  /// Shows screen `index` from the top
  fn show(&mut self, index: usize, ctx: &SystemContext) {
    if let Some(player) = &mut self.player {
      player.stop(ctx);
    }
    self.current = index;
    let Some(Some(scene)) = self.scenes.get(index) else {
      self.player = None;
      return;
    };
    let mut scene = PlayableScene::new(scene.clone());
    if let AttractScreen::LastScore(player) = self.screens[index]
      && let Some((label, score)) = self.last_scores.get(player)
    {
      scene = scene.variable("label", *label).variable("score", Value::Int(*score as i64));
    }
    self.player = Some(ScenePlayer::started(scene, ctx));
  }

  /// Moves on after a screen's time, counting from now
  fn cue_next(&mut self, ctx: &SystemContext) {
    ctx.cues.cancel(self.cue_id.take());
    self.cue_id = Some(ctx.cues.start(NextScreen, Cue::Forever(self.screen_duration)));
  }
}

impl System for DmdAttractSystem {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(GamePhase::None) && !ctx.systems.get::<DmdMenuSystem>().is_some_and(|menu| menu.is_active(ctx))
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    let content = content_with_bundled(ctx.content_path.clone());
    self.scenes = self
      .screens
      .iter()
      .map(|screen| match screen {
        AttractScreen::Scene(name) => open_scene(&content, name, Sends { variables: &[] }, self.z),
        AttractScreen::LastScore(_) => open_scene(&content, LAST_SCORE_SCENE, LAST_SCORE_SENDS, self.z),
        AttractScreen::PressStart => open_scene(&content, PRESS_START_SCENE, Sends { variables: &[] }, self.z),
      })
      .collect();
    self.cue_next(ctx);
  }

  fn on_reactivate(&mut self, ctx: &SystemContext) {
    self.cue_next(ctx);
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    ctx.cues.cancel(self.cue_id.take());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<NextScreen>() {
      self.step(1, ctx);
    } else if let Some(ended) = event.downcast_ref::<RoutineCompleted<GameEnding>>() {
      self.last_scores = ended.routine.scores.clone();
      // the scores the players finished with
      if let Some(first) = self.screens.iter().position(|screen| *screen == AttractScreen::LastScore(0)) {
        self.show(first, ctx);
        self.cue_next(ctx);
      }
    } else if let Some(closed) = event.downcast_ref::<SwitchClosed>()
      && let Some((previous, next)) = self.flippers
    {
      if closed.switch.name == previous {
        self.step(-1, ctx);
        self.cue_next(ctx);
      } else if closed.switch.name == next {
        self.step(1, ctx);
        self.cue_next(ctx);
      }
    }
  }

  /// A screen that stops having something to show (press start once a game can't be started) moves on
  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    if self.player.is_none() || !self.can_show(self.current, ctx) {
      if let Some(next) = self.showable_from(self.current, 1, ctx) {
        self.show(next, ctx);
      }
    }
  }
}

/// One of the screens [`DmdAttractSystem`] shows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractScreen {
  /// A scene from the content folder, by name
  Scene(&'static str),
  /// The score player `n` (from 0) finished the last game with: `frontbox/128x32/attract/last_score.scene`, sent the
  /// player's `label` and `score`. Skipped until a game has been played, and for a player who wasn't in it.
  LastScore(usize),
  /// `frontbox/128x32/attract/press_start.scene`, while a game can be started. Skipped otherwise.
  PressStart,
}

const LAST_SCORE_SENDS: Sends = Sends {
  variables: &[
    SentVariable { name: "label", sent_as: VariableType::Text, values: &[], about: "Who the score is for, e.g. PLAYER 1" },
    SentVariable { name: "score", sent_as: VariableType::Number, values: &[], about: "The score they finished with" },
  ],
};

#[derive(serde::Serialize, Event)]
struct NextScreen;

#[cfg(test)]
mod tests {
  use frontbox_scene::SceneSystem;

  use super::*;

  fn spawned(context: &mut TestContext, screens: Vec<AttractScreen>) -> DmdAttractSystem {
    context.insert_system(SceneSystem::new());
    let mut attract = DmdAttractSystem::new(screens);
    attract.on_spawn(&context.sys_ctx());
    attract.on_tick(Duration::ZERO, &context.sys_ctx());
    attract
  }

  const SCREENS: [AttractScreen; 3] = [AttractScreen::LastScore(0), AttractScreen::PressStart, AttractScreen::LastScore(1)];

  #[test]
  fn screens_with_nothing_to_show_are_skipped() {
    let mut context = TestContext::default();
    let attract = spawned(&mut context, SCREENS.to_vec());
    // no game has been played and none can be started
    assert!(attract.player.is_none());
  }

  #[test]
  fn a_game_ending_shows_its_first_score_then_steps_through_the_rest() {
    let mut context = TestContext::default();
    let mut attract = spawned(&mut context, SCREENS.to_vec());
    let ending = GameEnding { scores: vec![("PLAYER 1", 100), ("PLAYER 2", 200)], reason: crate::GameEndReason::Completed };
    attract.on_event(&RoutineCompleted { routine: ending, completion: RoutineCompletion::AllSteps }, &context.sys_ctx());
    assert_eq!(attract.current, 0);
    attract.step(1, &context.sys_ctx());
    assert_eq!(attract.current, 2);
    attract.step(1, &context.sys_ctx());
    assert_eq!(attract.current, 0);
    attract.step(-1, &context.sys_ctx());
    assert_eq!(attract.current, 2);
  }

  #[test]
  fn the_bundled_scenes_fit_what_is_sent() {
    let content = content_with_bundled(None);
    assert_eq!(LAST_SCORE_SENDS.check(&Scene::open(&content, LAST_SCORE_SCENE).unwrap()), []);
  }
}
