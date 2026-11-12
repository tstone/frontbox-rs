//! Whole games played on virtual hardware: balls served, plunged, drained and saved, as the systems that run a game
//! work it out between them.

use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_pinball::*;

// The machine: a three-ball trough ejecting into a plunge lane, a start button, a playfield switch, a tilt bob, and a
// lock that starts a multiball.

const TROUGH: [&str; 3] = ["trough_1", "trough_2", "trough_3"];
const EJECT: &str = "trough_eject";
const PLUNGE: &str = "plunge";
const START: &str = "start";
const PLAYFIELD: &str = "playfield";
const TILT_BOB: &str = "tilt_bob";
const LOCK: &str = "lock";

static TROUGH_SWITCHES: LazyLock<Vec<SwitchDefinition>> = LazyLock::new(|| {
  TROUGH
    .iter()
    .map(|name| TroughSystem::switch_definition(name).build())
    .collect()
});
static PLUNGE_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| PlungeLaneSystem::switch_definition(PLUNGE).build());
static START_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(START).build());
static PLAYFIELD_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(PLAYFIELD).build());
static TILT_BOB_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(TILT_BOB).build());
static LOCK_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(LOCK).build());
static EJECT_COIL: LazyLock<DriverDefinition> = LazyLock::new(|| TroughSystem::eject_coil_definition(EJECT).build());

fn app() -> App {
  let mut board = IoBoard::custom(16, 8)
    .wire_switch(3, &PLUNGE_SWITCH)
    .wire_switch(4, &START_SWITCH)
    .wire_switch(5, &PLAYFIELD_SWITCH)
    .wire_switch(6, &TILT_BOB_SWITCH)
    .wire_switch(7, &LOCK_SWITCH)
    .wire_driver(0, &EJECT_COIL);
  for (pin, switch) in TROUGH_SWITCHES.iter().enumerate() {
    board = board.wire_switch(pin as u16, switch);
  }

  let mut app = App::new(BootConfig {
    io_network: IoNetwork::new(vec![board]),
    config_path: None,
    metrics_path: None,
    content_path: None,
    ..BootConfig::default()
  });
  app
    .system(BallManager::standard(TROUGH, EJECT).plunge_lane(PLUNGE).build())
    .system(GameManager::competitive(
      1,
      vec![
        BallSaveSystem::new().into(),
        TiltSystem::new(SwitchQ::name(TILT_BOB)).into(),
        Multiball::default().into(),
      ],
      SwitchQ::name(PLAYFIELD),
    ))
    .system(FreePlay::new(SwitchQ::name(START)))
    .routine::<BallDraining>(routine![BallSaveSystem]);
  app
}

/// A two-ball multiball, as a game's mode would write one: hitting the lock puts two more balls into play with a ball
/// save, and it's over when a drain that stands leaves one ball in play.
#[derive(Clone, Default)]
struct Multiball {
  running: bool,
}

#[derive(serde::Serialize, Event)]
struct MultiballEnded;

const MULTIBALL_SAVE: Duration = Duration::from_secs(10);

impl System for Multiball {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>()
      && switch.name == LOCK
      && !self.running
    {
      self.running = true;
      ctx.eject_balls(2, Launch::Auto);
      ctx.request_ball_save(MULTIBALL_SAVE);
    } else if let Some(drain) = event.downcast_ref::<RoutineCompleted<BallDraining>>()
      && drain.completion == RoutineCompletion::AllSteps
      && drain.routine.in_play <= 1
      && self.running
    {
      self.running = false;
      ctx.emit(MultiballEnded);
    }
  }
}

/// The app running on the machine, with its balls moved the way they would be there. The trough only counts how many
/// of its switches are closed, so which ones a ball leaves or comes back to doesn't matter.
struct Game {
  scenario: Scenario,
  in_trough: usize,
}

impl Game {
  fn start() -> Self {
    Self {
      scenario: Scenario::new(app()).closed(TROUGH).start(),
      in_trough: TROUGH.len(),
    }
  }

  /// The ball the trough ejected rolls up into the plunge lane
  fn eject_into_lane(&mut self) {
    self.in_trough -= 1;
    self.scenario.open_switch(TROUGH[self.in_trough]);
    self.scenario.close_switch(PLUNGE);
    self.scenario.advance(ms(400)); // the trough settles
  }

  /// The player plunges the ball in the lane, and it doesn't roll back
  fn plunge(&mut self) {
    self.scenario.open_switch(PLUNGE);
    self.scenario.advance(ms(1100)); // past the lane's re-enter timeout
  }

  /// The ball the trough ejected is plunged into play
  fn serve(&mut self) {
    self.eject_into_lane();
    self.plunge();
  }

  /// A ball in play drains into the trough
  fn drain(&mut self) {
    self.scenario.close_switch(TROUGH[self.in_trough]);
    self.in_trough += 1;
    self.scenario.advance(ms(400)); // the trough settles
  }

  /// Play on until the turn's ball save, and its grace period, are over
  fn outlast_ball_save(&mut self) {
    self.scenario.advance(Duration::from_secs(11));
  }

  fn ejects(&self) -> u32 {
    self.hardware().driver(EJECT).unwrap().taps
  }
}

impl Deref for Game {
  type Target = Scenario;

  fn deref(&self) -> &Scenario {
    &self.scenario
  }
}

impl DerefMut for Game {
  fn deref_mut(&mut self) -> &mut Scenario {
    &mut self.scenario
  }
}

fn ms(ms: u64) -> Duration {
  Duration::from_millis(ms)
}

#[test]
fn a_drain_during_ball_save_is_saved_and_the_next_one_ends_the_turn() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let active = g.record(|e: &TurnActive| e.turn);
  let saves = g.record(|e: &BallSaved| e.count);
  let drains = g.record(|e: &RoutineCompleted<BallDraining>| e.completion);

  // Start a game: the first turn begins, and the trough serves a ball
  g.hit_switch(START);
  assert_eq!(turns.take(), [0]);
  assert_eq!(g.ejects(), 1);

  // It's plunged into play, which makes the turn active and starts the ball save
  g.eject_into_lane();
  assert!(active.take().is_empty(), "the ball is still in the lane");
  g.plunge();
  assert_eq!(active.take(), [0]);

  // It drains straight away, so ball save puts another ball into play and the drain doesn't stand
  g.drain();
  assert_eq!(saves.take(), [1]);
  assert_eq!(drains.take(), [RoutineCompletion::Halted]);
  assert_eq!(g.ejects(), 2);
  assert!(turns.take().is_empty(), "the turn goes on");

  g.serve();
  assert!(active.take().is_empty(), "a turn is only made active once");

  // Once the ball save and its grace period run out, a drain stands, ending the turn, and the next ball is served
  g.outlast_ball_save();
  g.drain();
  assert!(saves.take().is_empty());
  assert_eq!(drains.take(), [RoutineCompletion::AllSteps]);
  assert_eq!(turns.take(), [1]);
  assert_eq!(g.ejects(), 3);
}

#[test]
fn tilting_ends_the_turn_and_its_ball_cant_be_saved() {
  let mut g = Game::start();
  let warnings = g.record(|e: &TiltWarned| e.remaining);
  let turn_ends = g.record(|e: &RoutineStarted<TurnEnding>| e.routine.reason);
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let saves = g.record(|e: &BallSaved| e.count);

  g.hit_switch(START);
  g.serve();
  g.advance(ms(1000)); // past the bob's grace period after the serve

  // Each swing uses a warning, and swings while the bob settles don't count
  g.hit_switch(TILT_BOB);
  assert_eq!(warnings.take(), [1]);
  g.hit_switch(TILT_BOB);
  assert!(warnings.take().is_empty(), "the bob is still settling");
  g.advance(ms(1100));
  g.hit_switch(TILT_BOB);
  assert_eq!(warnings.take(), [0]);

  // With none left, the next swing tilts, ending the turn at once
  g.advance(ms(1100));
  g.hit_switch(TILT_BOB);
  assert!(warnings.take().is_empty());
  assert_eq!(turn_ends.take(), [TurnEndReason::Tilted]);
  assert_eq!(turns.take(), [0], "the next turn waits for the ball");

  // The ball save is still running, but the tilted ball rolls down unsaved, and the next turn begins
  g.drain();
  assert!(saves.take().is_empty());
  assert_eq!(turns.take(), [1]);
  assert_eq!(g.ejects(), 2);
}

#[test]
fn a_multiball_ends_on_the_drain_that_stands_with_one_ball_left() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  let drains = g.record(|e: &RoutineCompleted<BallDraining>| (e.completion, e.routine.in_play));
  let saves = g.record(|e: &BallSaved| e.count);
  let ended = g.record(|_: &MultiballEnded| ());
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);

  // The lock puts two more balls into play, one after the other
  g.hit_switch(LOCK);
  assert_eq!(g.ejects(), 2);
  g.serve();
  assert_eq!(g.ejects(), 3, "the second ball is ejected once the first leaves the lane");
  g.serve();

  // A drain under the multiball's ball save is saved, so it doesn't count toward ending the multiball
  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::Halted, 2)]);
  assert_eq!(saves.take(), [1]);
  assert_eq!(g.ejects(), 4);
  g.serve();

  // Once the save is over, drains stand: the multiball ends at one ball left, and the turn at none
  g.advance(MULTIBALL_SAVE + Duration::from_secs(2));
  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::AllSteps, 2)]);
  assert!(ended.take().is_empty());

  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::AllSteps, 1)]);
  assert_eq!(ended.take().len(), 1);
  assert!(turns.take().is_empty(), "one ball is still in play");

  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::AllSteps, 0)]);
  assert_eq!(turns.take(), [1]);
}

#[test]
fn the_game_ends_after_the_last_turn_and_another_can_start() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let endings = g.record(|e: &RoutineCompleted<GameEnding>| (e.routine.reason, e.routine.scores.len()));

  g.hit_switch(START);
  assert!(g.act(|ctx| ctx.states.is(GamePhase::Active)));

  for _ in 0..3 {
    g.serve();
    g.outlast_ball_save();
    g.drain();
  }
  assert_eq!(turns.take(), [0, 1, 2], "three turns by default");
  assert_eq!(endings.take(), [(GameEndReason::Completed, 1)]);
  assert!(g.act(|ctx| ctx.states.is(GamePhase::None)));
  assert_eq!(g.ejects(), 3, "no ball is served after the last turn");

  // The start button begins a new game
  g.hit_switch(START);
  assert!(g.act(|ctx| ctx.states.is(GamePhase::Active)));
  assert_eq!(turns.take(), [0]);
  assert_eq!(g.ejects(), 4);
}
