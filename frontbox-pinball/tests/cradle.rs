//! Cradling on virtual hardware: a three-ball trough ejecting into a plunge lane, two flipper buttons, and two
//! playfield switches (an inlane and a target).

use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_pinball::*;

const TROUGH: [&str; 3] = ["trough_1", "trough_2", "trough_3"];
const EJECT: &str = "trough_eject";
const PLUNGE: &str = "plunge";
const INLANE: &str = "inlane";
const TARGET: &str = "target";
const LEFT_FLIPPER: &str = "left_flipper";
const RIGHT_FLIPPER: &str = "right_flipper";

const SETTLE: Duration = Duration::from_secs(1);

static TROUGH_SWITCHES: LazyLock<Vec<SwitchDefinition>> = LazyLock::new(|| {
  TROUGH
    .iter()
    .map(|name| TroughSystem::switch_definition(name).build())
    .collect()
});
static PLUNGE_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| PlungeLaneSystem::switch_definition(PLUNGE).build());
static INLANE_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(INLANE).build());
static TARGET_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(TARGET).build());
static LEFT_FLIPPER_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(LEFT_FLIPPER).build());
static RIGHT_FLIPPER_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(RIGHT_FLIPPER).build());
static EJECT_COIL: LazyLock<DriverDefinition> =
  LazyLock::new(|| TroughSystem::eject_coil_definition(EJECT).build());

fn app() -> App {
  let mut board = IoBoard::custom(16, 8, DriverVoltage::High)
    .wire_switch(3, &PLUNGE_SWITCH)
    .wire_switch(4, &INLANE_SWITCH)
    .wire_switch(5, &TARGET_SWITCH)
    .wire_switch(6, &LEFT_FLIPPER_SWITCH)
    .wire_switch(7, &RIGHT_FLIPPER_SWITCH)
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
    .system(StandardBallManager::builder(TROUGH, EJECT).plunge_lane(PLUNGE).build())
    .system(CradleSystem::new(
      SwitchQ::name(INLANE).or(SwitchQ::name(TARGET)),
      SwitchQ::name(LEFT_FLIPPER).or(SwitchQ::name(RIGHT_FLIPPER)),
    ));
  app
}

/// The app running on the machine, with its balls moved the way they would be there
struct Machine {
  scenario: Scenario,
  in_trough: usize,
}

impl Machine {
  fn start() -> Self {
    Self {
      scenario: Scenario::new(app()).closed(TROUGH).start(),
      in_trough: TROUGH.len(),
    }
  }

  /// Asks for a ball, which the trough ejects into the plunge lane
  fn eject_into_lane(&mut self) {
    self.scenario.act(|ctx| ctx.eject_ball(Launch::Player));
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

  /// A ball is put into play
  fn serve(&mut self) {
    self.eject_into_lane();
    self.plunge();
  }

  fn drain(&mut self) {
    self.scenario.close_switch(TROUGH[self.in_trough]);
    self.in_trough += 1;
    self.scenario.advance(ms(400)); // the trough settles
  }

  fn hold(&mut self, held: u8) {
    self.scenario.act(|ctx| ctx.set_balls_held(held));
  }

  fn is_cradled(&mut self) -> bool {
    self.scenario.act(|ctx| ctx.states.is(Cradle::Cradled))
  }
}

impl Deref for Machine {
  type Target = Scenario;

  fn deref(&self) -> &Scenario {
    &self.scenario
  }
}

impl DerefMut for Machine {
  fn deref_mut(&mut self) -> &mut Scenario {
    &mut self.scenario
  }
}

fn ms(ms: u64) -> Duration {
  Duration::from_millis(ms)
}

/// A little past `duration`, so a timer due at it has fired
fn past(duration: Duration) -> Duration {
  duration + ms(50)
}

#[test]
fn a_flipper_held_with_no_playfield_hits_cradles_the_ball_once_it_settles() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(SETTLE - ms(100));
  assert!(!m.is_cradled());

  m.advance(ms(150));
  assert!(m.is_cradled());
}

#[test]
fn a_ball_rolling_down_the_inlane_onto_a_held_flipper_is_cradled_once_it_comes_to_rest() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(ms(500));
  m.hit_switch(INLANE);
  m.advance(ms(700));
  assert!(!m.is_cradled(), "the settle time starts over from the hit");

  m.advance(ms(350));
  assert!(m.is_cradled());
}

#[test]
fn releasing_the_flipper_ends_the_cradle() {
  let mut m = Machine::start();
  m.serve();
  let changes = m.record(|e: &StateChanged<Cradle>| e.to);

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));
  m.open_switch(LEFT_FLIPPER);
  m.advance(ms(10));

  assert_eq!(changes.take(), [Cradle::Cradled, Cradle::Free]);
}

#[test]
fn a_playfield_hit_ends_the_cradle_and_a_ball_back_at_rest_is_cradled_again() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));
  assert!(m.is_cradled());

  // the ball got away, hit a target, and came back down to the still-raised flipper
  m.hit_switch(TARGET);
  assert!(!m.is_cradled());

  m.advance(past(SETTLE));
  assert!(m.is_cradled());
}

#[test]
fn letting_go_of_one_flipper_while_both_are_held_ends_the_cradle() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));
  m.close_switch(RIGHT_FLIPPER);
  m.advance(ms(10));
  assert!(m.is_cradled(), "raising the other flipper doesn't end it");

  // the ball may have been on the released flipper
  m.open_switch(LEFT_FLIPPER);
  m.advance(ms(10));
  assert!(!m.is_cradled());

  // it rolls across to the right flipper and settles there
  m.advance(past(SETTLE));
  assert!(m.is_cradled());
}

#[test]
fn a_ball_waiting_in_the_plunge_lane_is_not_cradled() {
  let mut m = Machine::start();
  m.eject_into_lane();

  m.close_switch(LEFT_FLIPPER);
  m.advance(Duration::from_secs(5));
  assert!(!m.is_cradled());
}

#[test]
fn a_ball_held_by_the_machine_is_not_cradled() {
  let mut m = Machine::start();
  m.serve();
  m.hold(1);

  m.close_switch(LEFT_FLIPPER);
  m.advance(Duration::from_secs(5));
  assert!(!m.is_cradled());
}

#[test]
fn there_is_no_cradle_with_two_balls_loose() {
  let mut m = Machine::start();
  m.serve();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(Duration::from_secs(5));
  assert!(!m.is_cradled());

  // one drains, leaving the other on the flipper
  m.drain();
  m.advance(past(SETTLE));
  assert!(m.is_cradled());
}

#[test]
fn a_ball_held_by_the_machine_leaves_the_other_one_to_be_cradled_until_it_is_released() {
  let mut m = Machine::start();
  m.serve();
  m.serve();
  m.hold(1);

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));
  assert!(m.is_cradled());

  // the scoop kicks its ball out, and there are two loose
  m.hold(0);
  m.advance(ms(10));
  assert!(!m.is_cradled());

  m.advance(Duration::from_secs(5));
  assert!(!m.is_cradled());
}

#[test]
fn a_ball_put_into_play_ends_the_cradle() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));

  // an add-a-ball: the trough has settled on the ball leaving by the time it reaches the lane, and a ball waiting in
  // the lane isn't loose yet
  m.act(|ctx| ctx.eject_ball(Launch::Player));
  m.in_trough -= 1;
  let leaving = TROUGH[m.in_trough];
  m.open_switch(leaving);
  m.advance(ms(400));
  m.close_switch(PLUNGE);
  m.advance(ms(10));
  assert!(m.is_cradled());

  m.plunge();
  assert!(!m.is_cradled());
}

#[test]
fn a_ball_reaching_the_lane_before_the_trough_settles_is_cradled_again_once_it_has() {
  let mut m = Machine::start();
  m.serve();

  m.close_switch(LEFT_FLIPPER);
  m.advance(past(SETTLE));

  // ball management briefly counts the ball in the lane as the one in play, which ends the cradle
  m.eject_into_lane();
  assert!(!m.is_cradled());

  m.advance(SETTLE);
  assert!(m.is_cradled());
}
