//! Whole games played on virtual hardware: balls served, plunged, drained and saved, as the systems that run a game
//! work it out between them.

use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_pinball::*;

// The machine: a three-ball trough ejecting into a plunge lane, a start button, a playfield switch, a tilt bob, a
// lock that starts a multiball, a drop target bank and a pop bumper that a ball search fires, a coin door, and a
// flipper button.

const TROUGH: [&str; 3] = ["trough_1", "trough_2", "trough_3"];
const EJECT: &str = "trough_eject";
const PLUNGE: &str = "plunge";
const START: &str = "start";
const PLAYFIELD: &str = "playfield";
const TILT_BOB: &str = "tilt_bob";
const LOCK: &str = "lock";
const DROP_RESET: &str = "drop_reset";
const POP: &str = "pop";
const FLIPPER: &str = "flipper";
const AUTOPLUNGER: &str = "autoplunger";
/// Closed while the door is shut
const COIN_DOOR: &str = "coin_door";

static TROUGH_SWITCHES: LazyLock<Vec<SwitchDefinition>> = LazyLock::new(|| {
  TROUGH
    .iter()
    .map(|name| TroughSystem::switch_definition(name).build())
    .collect()
});
static PLUNGE_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| PlungeLaneSystem::switch_definition(PLUNGE).build());
static START_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(START).build());
static PLAYFIELD_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(PLAYFIELD).build());
static TILT_BOB_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(TILT_BOB).build());
static LOCK_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(LOCK).build());
static COIN_DOOR_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(COIN_DOOR).build());
static FLIPPER_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(FLIPPER).build());
static EJECT_COIL: LazyLock<DriverDefinition> =
  LazyLock::new(|| TroughSystem::eject_coil_definition(EJECT).build());
static DROP_RESET_COIL: LazyLock<DriverDefinition> = LazyLock::new(|| {
  DriverDefinitionBuilder::new(DROP_RESET)
    .mode(DriverMode::pulse_kick().build())
    .build()
});
static POP_COIL: LazyLock<DriverDefinition> = LazyLock::new(|| {
  DriverDefinitionBuilder::new(POP)
    .mode(DriverMode::pulse().build())
    .build()
});
static AUTOPLUNGER_COIL: LazyLock<DriverDefinition> =
  LazyLock::new(|| AutoPlungerSystem::coil_definition(AUTOPLUNGER).build());

fn app() -> App {
  app_with(
    BallManager::standard(TROUGH, EJECT)
      .plunge_lane(PLUNGE)
      .build(),
  )
}

/// The machine with an autoplunger in its plunge lane, and no launch button
fn app_with_autoplunger() -> App {
  app_with(
    BallManager::standard(TROUGH, EJECT)
      .plunge_lane(PLUNGE)
      .autoplunger(AUTOPLUNGER)
      .build(),
  )
}

fn app_with(ball_manager: BallManager) -> App {
  let mut board = IoBoard::custom(16, 8)
    .wire_switch(3, &PLUNGE_SWITCH)
    .wire_switch(4, &START_SWITCH)
    .wire_switch(5, &PLAYFIELD_SWITCH)
    .wire_switch(6, &TILT_BOB_SWITCH)
    .wire_switch(7, &LOCK_SWITCH)
    .wire_switch(8, &COIN_DOOR_SWITCH)
    .wire_switch(9, &FLIPPER_SWITCH)
    .wire_driver(0, &EJECT_COIL)
    .wire_driver(1, &DROP_RESET_COIL)
    .wire_driver(2, &POP_COIL)
    .wire_driver(3, &AUTOPLUNGER_COIL);
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
    .system(ball_manager)
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
    .system(
      BallSearchSystem::new(SwitchQ::name(PLAYFIELD))
        .hold_switches(SwitchQ::name(FLIPPER))
        .coils(DriverQ::name(POP)),
    )
    .system(DropBank)
    .system(CoinDoorSystem::new(COIN_DOOR))
    .routine::<BallDraining>(routine![BallSaveSystem])
    .routine::<BallSearching>(routine![DropBank]);
  app
}

/// A drop target bank. A ball search resets it, which knocks loose a ball resting on a standing target.
#[derive(Clone)]
struct DropBank;

impl System for DropBank {
  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    if routine.is::<BallSearching>() {
      ctx.drivers.activate(DROP_RESET, ActivationMode::Tap);
      ctx.routines.complete_step::<BallSearching>();
    }
  }
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
      scenario: Scenario::new(app())
        .closed(TROUGH.iter().copied().chain([COIN_DOOR]))
        .start(),
      in_trough: TROUGH.len(),
    }
  }

  fn start_with_autoplunger() -> Self {
    Self {
      scenario: Scenario::new(app_with_autoplunger())
        .closed(TROUGH.iter().copied().chain([COIN_DOOR]))
        .start(),
      in_trough: TROUGH.len(),
    }
  }

  /// Boots with one ball left in the plunge lane, e.g. from a game the machine was switched off during
  fn start_with_ball_in_lane() -> Self {
    let in_trough = TROUGH.len() - 1;
    let closed = TROUGH[..in_trough]
      .iter()
      .copied()
      .chain([PLUNGE, COIN_DOOR]);
    Self {
      scenario: Scenario::new(app()).closed(closed).start(),
      in_trough,
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

  fn autoplunges(&self) -> u32 {
    self.hardware().driver(AUTOPLUNGER).unwrap().taps
  }

  fn drop_resets(&self) -> u32 {
    self.hardware().driver(DROP_RESET).unwrap().taps
  }

  fn pops(&self) -> u32 {
    self.hardware().driver(POP).unwrap().taps
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
  assert_eq!(
    g.ejects(),
    3,
    "the second ball is ejected once the first leaves the lane"
  );
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
  let endings =
    g.record(|e: &RoutineCompleted<GameEnding>| (e.routine.reason, e.routine.scores.len()));

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

#[test]
fn a_ball_left_in_the_lane_at_boot_serves_the_first_turn() {
  let mut g = Game::start_with_ball_in_lane();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let drains = g.record(|e: &RoutineCompleted<BallDraining>| (e.completion, e.routine.in_play));

  g.hit_switch(START);
  assert_eq!(turns.take(), [0]);
  assert_eq!(g.ejects(), 0, "the lane ball is served");

  // It's counted, so its drain ends the turn
  g.plunge();
  g.outlast_ball_save();
  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::AllSteps, 0)]);
  assert_eq!(turns.take(), [1]);
  assert_eq!(g.ejects(), 1);
}

#[test]
fn tilting_with_the_ball_in_the_lane_leaves_it_for_the_next_turn() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let active = g.record(|e: &TurnActive| e.turn);

  g.hit_switch(START);
  g.eject_into_lane();
  g.act(|ctx| ctx.tilt());

  // Nothing is in play, so the next turn starts at once, and is served the ball still in the lane
  assert_eq!(turns.take(), [0, 1]);
  assert_eq!(g.ejects(), 1);

  g.plunge();
  assert_eq!(active.take(), [1]);
  g.outlast_ball_save();
  g.drain();
  assert_eq!(turns.take(), [2]);
  assert_eq!(g.ejects(), 2);
}

#[test]
fn tilting_during_a_multiball_start_stops_the_balls_still_to_come() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  // The lock asks for two balls; the first is in the lane when the player tilts
  g.hit_switch(LOCK);
  g.eject_into_lane();
  assert_eq!(g.ejects(), 2);
  g.act(|ctx| ctx.tilt());

  // The ball on the playfield drains, and the next turn is served the one left in the lane. The second multiball
  // ball is never ejected.
  g.drain();
  assert_eq!(turns.take(), [0, 1]);
  assert_eq!(g.ejects(), 2);
}

#[test]
fn quitting_with_the_ball_in_the_lane_leaves_it_for_the_next_game() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let drains = g.record(|e: &RoutineCompleted<BallDraining>| (e.completion, e.routine.in_play));

  g.hit_switch(START);
  g.eject_into_lane();
  g.act(|ctx| ctx.quit_game(GameEndReason::Quit));
  assert!(g.act(|ctx| ctx.states.is(GamePhase::None)));

  // The new game is served the ball still in the lane, rather than a second one behind it
  g.hit_switch(START);
  assert_eq!(turns.take(), [0, 0]);
  assert_eq!(g.ejects(), 1);

  g.plunge();
  g.outlast_ball_save();
  g.drain();
  assert_eq!(drains.take(), [(RoutineCompletion::AllSteps, 0)]);
  assert_eq!(turns.take(), [1]);
  assert_eq!(g.ejects(), 2);
}

#[test]
fn after_quitting_a_multiball_another_game_waits_for_the_balls_to_come_home() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();
  g.hit_switch(LOCK);
  g.serve();
  g.serve();
  turns.take();

  g.act(|ctx| ctx.quit_game(GameEndReason::Quit));
  assert!(g.act(|ctx| ctx.states.is(GamePhase::None)));
  assert!(
    g.act(|ctx| ctx.states.is(PlayerEntry::Closed)),
    "three balls are still in play"
  );

  g.drain();
  g.drain();
  assert!(g.act(|ctx| ctx.states.is(PlayerEntry::Closed)));
  g.drain();
  assert!(g.act(|ctx| ctx.states.is(PlayerEntry::Open)));

  g.hit_switch(START);
  assert_eq!(turns.take(), [0]);
  assert_eq!(g.ejects(), 4);
}

#[test]
fn a_ball_that_never_comes_home_stops_being_waited_for() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  g.act(|ctx| ctx.quit_game(GameEndReason::Quit));
  assert!(g.act(|ctx| ctx.states.is(PlayerEntry::Closed)));

  g.advance(Duration::from_secs(31));
  assert!(g.act(|ctx| ctx.states.is(PlayerEntry::Open)));
}

#[test]
fn a_served_ball_that_drains_before_the_turn_goes_active_is_served_again() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let drains = g.record(|e: &RoutineStarted<BallDraining>| e.routine.count);

  // A weak plunge goes straight down the drain, before the lane counts the ball as gone
  g.hit_switch(START);
  g.eject_into_lane();
  g.open_switch(PLUNGE);
  g.drain();
  assert!(drains.take().is_empty(), "not a drain of the turn's ball");
  g.advance(ms(1000));
  assert_eq!(g.ejects(), 2, "served again");

  g.serve();
  g.outlast_ball_save();
  g.drain();
  assert_eq!(turns.take(), [0, 1], "the turn wasn't used up");
}

#[test]
fn a_turn_with_nothing_in_play_ends_even_without_a_drain_reported() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();

  // The ball save's replacement jams in the trough: the eject fails, leaving nothing in play and no drain to report
  g.drain();
  assert_eq!(g.ejects(), 2);
  g.advance(Duration::from_secs(20));
  assert_eq!(turns.take(), [0, 1]);
}

#[test]
fn a_held_ball_is_not_taken_for_a_drain() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  // A scoop holds the only ball, for a mode's intro say
  g.act(|ctx| ctx.set_balls_held(1));
  g.advance(Duration::from_secs(5));
  assert_eq!(turns.take(), [0], "the turn goes on");

  // Kicked back out, it's in play again, and its drain ends the turn
  g.act(|ctx| ctx.set_balls_held(0));
  g.drain();
  assert_eq!(turns.take(), [1]);
}

#[test]
fn a_ball_stuck_after_a_tilt_is_searched_for_and_the_game_goes_on() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();
  g.act(|ctx| ctx.tilt());
  assert_eq!(turns.take(), [0], "the next turn waits for the ball");

  // The ball comes to rest on a drop target, so nothing is hit until the search resets the bank
  g.advance(Duration::from_secs(14));
  assert_eq!(g.drop_resets(), 0);
  g.advance(Duration::from_secs(2));
  assert_eq!(g.drop_resets(), 1);

  // Knocked loose, it drains, and the next turn begins
  g.drain();
  assert_eq!(turns.take(), [1]);
  assert_eq!(g.drop_resets(), 1);
}

#[test]
fn a_ball_search_fires_its_coils_one_at_a_time() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  while g.drop_resets() == 0 {
    g.advance(ms(50));
  }

  // The pop waits for the drop bank's coil to recover
  assert_eq!(g.pops(), 0);
  g.advance(ms(300));
  assert_eq!(g.pops(), 1);
  g.advance(ms(300));
  assert_eq!((g.drop_resets(), g.pops()), (1, 1), "once per round");
}

#[test]
fn a_ball_a_search_cant_find_is_replaced_and_the_turn_goes_on() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let failed = g.record(|e: &BallSearchFailed| (e.rounds, e.in_play));
  g.hit_switch(START);
  g.serve();

  // Three rounds find nothing, so the ball is given up on and another put into play, without using up the turn
  g.advance(Duration::from_secs(27));
  assert_eq!(g.drop_resets(), 3);
  assert!(failed.take().is_empty(), "the last round gets its pause too");
  g.advance(Duration::from_secs(5));
  assert_eq!(failed.take(), [(3, 0)]);
  assert_eq!(g.ejects(), 2);
  assert_eq!(turns.take(), [0]);

  g.serve();
  g.drain();
  assert_eq!(turns.take(), [1]);
}

#[test]
fn a_ball_knocked_loose_by_the_last_round_is_not_given_up_on() {
  let mut g = Game::start();
  let failed = g.record(|e: &BallSearchFailed| e.rounds);
  g.hit_switch(START);
  g.serve();
  while g.drop_resets() < 3 {
    g.advance(ms(50));
  }

  g.advance(ms(1000));
  g.hit_switch(PLAYFIELD);
  g.advance(Duration::from_secs(10));
  assert!(failed.take().is_empty());
  assert_eq!(g.ejects(), 1, "no replacement");
}

#[test]
fn balls_stuck_together_are_given_up_on_one_search_after_another() {
  let mut g = Game::start();
  let failed = g.record(|e: &BallSearchFailed| e.in_play);
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();
  g.hit_switch(LOCK);
  g.serve();
  g.serve();

  // All three wedge somewhere. Each search gives up on one, and the next goes looking for the rest.
  g.advance(Duration::from_secs(35));
  assert_eq!(failed.take(), [2]);
  g.advance(Duration::from_secs(70));
  assert_eq!(failed.take(), [1, 0]);
}

#[test]
fn a_held_ball_being_released_puts_off_a_ball_search() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();

  // A scoop holds the ball for most of the delay, then kicks it out
  g.act(|ctx| ctx.set_balls_held(1));
  g.advance(Duration::from_secs(14));
  g.act(|ctx| ctx.set_balls_held(0));
  g.advance(Duration::from_secs(2));
  assert_eq!(g.drop_resets(), 0, "it hasn't had the chance to hit a switch");
  g.advance(Duration::from_secs(14));
  assert_eq!(g.drop_resets(), 1);
}

#[test]
fn a_ball_lost_after_a_tilt_does_not_hold_up_the_next_turn() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.hit_switch(START);
  g.serve();
  g.act(|ctx| ctx.tilt());

  g.advance(Duration::from_secs(40));
  assert_eq!(g.drop_resets(), 3);
  assert_eq!(turns.take(), [0, 1]);
}

#[test]
fn playfield_switch_hits_put_off_a_ball_search() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  g.advance(Duration::from_secs(10));
  g.hit_switch(PLAYFIELD);
  g.advance(Duration::from_secs(10));
  assert_eq!(g.drop_resets(), 0);
  g.advance(Duration::from_secs(6));
  assert_eq!(g.drop_resets(), 1);
}

#[test]
fn holding_a_flipper_puts_off_a_ball_search() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();

  // The player cradles the ball for a long time
  g.close_switch(FLIPPER);
  g.advance(Duration::from_secs(30));
  assert_eq!(g.drop_resets(), 0);

  // Letting go starts the delay over
  g.open_switch(FLIPPER);
  g.advance(Duration::from_secs(14));
  assert_eq!(g.drop_resets(), 0);
  g.advance(Duration::from_secs(2));
  assert_eq!(g.drop_resets(), 1);
}

#[test]
fn a_ball_waiting_in_the_plunge_lane_is_not_searched_for() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.eject_into_lane();
  g.advance(Duration::from_secs(60));
  assert_eq!(g.drop_resets(), 0);
}

#[test]
fn a_drain_with_the_coin_door_open_is_put_back_into_play() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  let saves = g.record(|e: &BallSaved| e.count);
  let drains = g.record(|e: &RoutineStarted<BallDraining>| e.routine.in_play);
  g.hit_switch(START);
  g.serve();

  // An operator opens the door and takes out a stuck ball, during the ball save: it's neither saved nor a drain
  g.open_switch(COIN_DOOR);
  g.drain();
  assert!(drains.take().is_empty());
  assert!(saves.take().is_empty());
  assert_eq!(g.ejects(), 2);
  assert_eq!(turns.take(), [0], "the turn goes on");

  // With the door closed, drains count again
  g.serve();
  g.close_switch(COIN_DOOR);
  g.outlast_ball_save();
  g.drain();
  assert_eq!(drains.take(), [0]);
  assert_eq!(turns.take(), [1]);
}

#[test]
fn a_drain_with_the_coin_door_open_counts_with_the_setting_off() {
  let mut g = Game::start();
  let turns = g.record(|e: &RoutineStarted<TurnStarting>| e.routine.turn);
  g.act(|ctx| {
    ctx
      .config
      .set(&configs::IGNORE_DRAINS_WITH_COIN_DOOR_OPEN, false)
  });
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  g.open_switch(COIN_DOOR);
  g.drain();
  assert_eq!(turns.take(), [0, 1]);
}

#[test]
fn there_is_no_ball_search_while_the_coin_door_is_open() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  g.open_switch(COIN_DOOR);
  g.advance(Duration::from_secs(60));
  assert_eq!(g.drop_resets(), 0);

  // Closing the door starts the delay over
  g.close_switch(COIN_DOOR);
  g.advance(Duration::from_secs(14));
  assert_eq!(g.drop_resets(), 0);
  g.advance(Duration::from_secs(2));
  assert_eq!(g.drop_resets(), 1);

  // Opening it partway through a search ends the search
  g.open_switch(COIN_DOOR);
  g.advance(Duration::from_secs(30));
  assert_eq!(g.drop_resets(), 1);
}

#[test]
fn a_ball_found_stops_the_round_in_progress() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  while g.drop_resets() == 0 {
    g.advance(ms(50));
  }

  // Knocked loose by the drop bank, so the pop has no need to fire
  g.hit_switch(PLAYFIELD);
  g.advance(Duration::from_secs(5));
  assert_eq!((g.drop_resets(), g.pops()), (1, 0));
}

#[test]
fn opening_the_coin_door_stops_the_round_in_progress() {
  let mut g = Game::start();
  g.hit_switch(START);
  g.serve();
  while g.drop_resets() == 0 {
    g.advance(ms(50));
  }

  // The pop would fire next, but an operator may have a hand on the playfield
  g.open_switch(COIN_DOOR);
  g.advance(Duration::from_secs(5));
  assert_eq!((g.drop_resets(), g.pops()), (1, 0));
}

#[test]
fn an_autoplunge_that_doesnt_move_the_ball_is_repeated() {
  let mut g = Game::start_with_autoplunger();
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  // The lock asks for two balls. The first is still on the lane switch after the autoplunger fires (e.g. it wasn't
  // seated yet), so the autoplunger fires again.
  g.hit_switch(LOCK);
  g.eject_into_lane();
  assert_eq!(g.autoplunges(), 1);
  g.advance(ms(1100));
  assert_eq!(g.autoplunges(), 2);

  // That launches it, and the second ball is ejected behind it
  g.plunge();
  assert_eq!(g.ejects(), 3);
  g.advance(Duration::from_secs(5));
  assert_eq!(g.autoplunges(), 2);
}

#[test]
fn an_autoplunger_that_cant_move_the_ball_stops_firing() {
  let mut g = Game::start_with_autoplunger();
  g.hit_switch(START);
  g.serve();
  g.outlast_ball_save();

  g.hit_switch(LOCK);
  g.eject_into_lane();
  g.advance(Duration::from_secs(30));
  assert_eq!(g.autoplunges(), 4, "the first launch and three retries");
}
