//! Two scoops on virtual hardware: one with a lower entrance and an eject scene, whose ball can be held, and a plain one.

use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_led::prelude::*;
use frontbox_pinball::*;

struct Lower;
struct Rear;

const LOWER: &str = "lower_scoop";
const LOWER_COIL: &str = "lower_scoop_coil";
const LOWER_ENTRANCE: &str = "lower_entrance";
const REAR: &str = "rear_scoop";
const REAR_COIL: &str = "rear_scoop_coil";

/// How long the lower scoop's eject scene is
const EJECT_DELAY: Duration = Duration::from_millis(750);
const VERIFY_DELAY: Duration = Duration::from_millis(250);
const LOWER_ENTRANCE_TIMEOUT: Duration = Duration::from_secs(3);

static LOWER_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(LOWER).build());
static LOWER_ENTRANCE_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(LOWER_ENTRANCE).build());
static REAR_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(REAR).build());
static LOWER_DRIVER: LazyLock<DriverDefinition> = LazyLock::new(|| {
  DriverDefinitionBuilder::new(LOWER_COIL)
    .mode(DriverMode::pulse_kick().build())
    .build()
});
static REAR_DRIVER: LazyLock<DriverDefinition> = LazyLock::new(|| {
  DriverDefinitionBuilder::new(REAR_COIL)
    .mode(DriverMode::pulse_kick().build())
    .build()
});

/// A scene with the given timing line, on an LED the machine doesn't have
fn scene(timing: &str) -> SceneContext<Completes> {
  let scene = format!(
    r#"Scene(
      version: 2,
      {timing}
      layers: [Leds(leds: Name("scoop_led"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
    )"#
  );
  SceneContext::completing(Scene::load(&scene).unwrap()).unwrap()
}

fn app() -> App {
  let mut app = App::new(BootConfig {
    io_network: IoNetwork::new(vec![
      IoBoard::custom(8, 8)
        .wire_switch(0, &LOWER_SWITCH)
        .wire_switch(1, &LOWER_ENTRANCE_SWITCH)
        .wire_switch(2, &REAR_SWITCH)
        .wire_driver(0, &LOWER_DRIVER)
        .wire_driver(1, &REAR_DRIVER),
    ]),
    config_path: None,
    metrics_path: None,
    content_path: None,
    ..BootConfig::default()
  });
  app
    .system(
      ScoopSystem::<Lower>::new(LOWER, LOWER_COIL)
        .lower_entrance_switch(LOWER_ENTRANCE)
        .lower_entrance_timeout(LOWER_ENTRANCE_TIMEOUT)
        .before_eject_scene(scene("timing: (duration: 0.25, cycle: Times(3)),"))
        .verify_delay(VERIFY_DELAY),
    )
    .system(ScoopSystem::<Rear>::new(REAR, REAR_COIL).verify_delay(VERIFY_DELAY));
  app.routine::<BallSearching>(routine![ScoopSystem<Lower>, ScoopSystem<Rear>]);
  app
}

fn taps(s: &Scenario, coil: &str) -> u32 {
  s.hardware().driver(coil).unwrap().taps
}

fn hold_lower(s: &mut Scenario, requester: u64) {
  s.act(|ctx| {
    ctx
      .systems
      .expect::<ScoopSystem<Lower>>()
      .hold(requester, ctx.into())
  });
}

fn release_lower(s: &mut Scenario, requester: u64) {
  s.act(|ctx| {
    ctx
      .systems
      .expect::<ScoopSystem<Lower>>()
      .release(requester, ctx.into())
  });
}

/// A little past `duration`, so a cue due at it has fired
fn past(duration: Duration) -> Duration {
  duration + Duration::from_millis(50)
}

#[test]
fn a_ball_is_ejected_once_the_eject_scene_has_played() {
  let mut s = Scenario::new(app()).start();
  let entered = s.record(|e: &ScoopBallEntered<Lower>| e.entry);
  let started = s.record(|_: &ScoopEjectStarted<Lower>| ());
  let ejected = s.record(|_: &ScoopEjected<Lower>| ());

  s.close_switch(LOWER);
  assert_eq!(entered.take(), [ScoopEntry::Direct]);
  assert_eq!(started.take().len(), 1);
  assert_eq!(taps(&s, LOWER_COIL), 0);

  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 1);
  assert_eq!(ejected.take().len(), 1);
}

#[test]
fn with_no_eject_scene_the_ball_is_ejected_at_once() {
  let mut s = Scenario::new(app()).start();
  let ejected = s.record(|_: &ScoopEjected<Rear>| ());

  s.close_switch(REAR);
  assert_eq!(taps(&s, REAR_COIL), 1);
  assert_eq!(ejected.take().len(), 1);
}

#[test]
fn each_scoop_only_hears_its_own_events() {
  let mut s = Scenario::new(app()).start();
  let lower_entered = s.record(|_: &ScoopBallEntered<Lower>| ());
  let lower_ejected = s.record(|_: &ScoopEjected<Lower>| ());

  s.close_switch(REAR);
  s.advance(past(EJECT_DELAY));
  assert!(lower_entered.take().is_empty());
  assert!(lower_ejected.take().is_empty());
  assert_eq!(taps(&s, LOWER_COIL), 0);
}

#[test]
fn a_ball_still_there_after_the_kick_is_kicked_again() {
  let mut s = Scenario::new(app()).start();
  let ejected = s.record(|_: &ScoopEjected<Rear>| ());

  s.close_switch(REAR);
  s.advance(past(VERIFY_DELAY));
  assert_eq!(taps(&s, REAR_COIL), 2);

  s.open_switch(REAR);
  s.advance(past(VERIFY_DELAY));
  assert_eq!(taps(&s, REAR_COIL), 2, "it left, so no more kicks");
  assert_eq!(ejected.take().len(), 1, "once per ball");
}

#[test]
fn a_ball_in_the_scoop_at_boot_is_kicked_out() {
  let s = Scenario::new(app()).closed([REAR]).start();
  assert_eq!(taps(&s, REAR_COIL), 1);
}

#[test]
fn a_ball_that_passed_the_lower_entrance_switch_entered_that_way() {
  let mut s = Scenario::new(app()).start();
  let entered = s.record(|e: &ScoopBallEntered<Lower>| e.entry);

  s.hit_switch(LOWER_ENTRANCE);
  s.advance(Duration::from_secs(1));
  s.close_switch(LOWER);
  assert_eq!(entered.take(), [ScoopEntry::LowerEntrance]);

  // the next one came in the front
  s.advance(past(EJECT_DELAY));
  s.open_switch(LOWER);
  s.close_switch(LOWER);
  assert_eq!(entered.take(), [ScoopEntry::Direct]);
}

#[test]
fn a_lower_entrance_hit_that_never_arrives_expires() {
  let mut s = Scenario::new(app()).start();
  let entered = s.record(|e: &ScoopBallEntered<Lower>| e.entry);

  s.hit_switch(LOWER_ENTRANCE);
  s.advance(past(LOWER_ENTRANCE_TIMEOUT));
  s.close_switch(LOWER);
  assert_eq!(entered.take(), [ScoopEntry::Direct]);
}

#[test]
fn a_held_ball_waits_until_the_last_hold_is_released() {
  let mut s = Scenario::new(app()).start();
  let ejected = s.record(|_: &ScoopEjected<Lower>| ());

  s.close_switch(LOWER);
  hold_lower(&mut s, 1);
  hold_lower(&mut s, 2);
  s.advance(Duration::from_secs(10));
  assert_eq!(taps(&s, LOWER_COIL), 0);

  release_lower(&mut s, 1);
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 0, "still held by the other");

  release_lower(&mut s, 2);
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 1);
  assert_eq!(ejected.take().len(), 1);
}

#[test]
fn a_hold_during_the_eject_scene_stops_the_eject() {
  let mut s = Scenario::new(app()).start();
  let started = s.record(|_: &ScoopEjectStarted<Lower>| ());

  s.close_switch(LOWER);
  s.advance(EJECT_DELAY / 2);
  hold_lower(&mut s, 1);
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 0);

  release_lower(&mut s, 1);
  assert_eq!(started.take().len(), 2, "the eject starts over");
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 1);
}

#[test]
fn a_tilt_clears_every_hold() {
  let mut s = Scenario::new(app()).start();
  s.close_switch(LOWER);
  hold_lower(&mut s, 1);
  hold_lower(&mut s, 2);

  s.emit(PlayerTilted { player_index: 0 });
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 1);
}

#[test]
fn a_despawned_systems_hold_is_cleared() {
  let mut s = Scenario::new(app()).start();
  s.close_switch(LOWER);
  hold_lower(&mut s, 1);

  s.emit(SystemDespawned::new(1, ROOT_GROUP));
  s.advance(past(EJECT_DELAY));
  assert_eq!(taps(&s, LOWER_COIL), 1);
}

#[test]
fn a_ball_search_kicks_empty_scoops_but_not_a_held_ball() {
  let mut s = Scenario::new(app()).start();
  s.close_switch(LOWER);
  hold_lower(&mut s, 1);

  s.act(|ctx| ctx.routines.start(BallSearching { round: 1 }));
  s.advance(Duration::from_secs(2));
  assert_eq!(taps(&s, LOWER_COIL), 0);
  assert_eq!(taps(&s, REAR_COIL), 1);
}
