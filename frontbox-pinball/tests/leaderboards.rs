//! Initials entry when a game ends, on virtual hardware

use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_lighting::LayeredLightManager;
use frontbox_pinball::*;

// The machine: a two-ball trough, a start button, and two flipper buttons. Its one leaderboard has no defaults, so any
// score makes it.

const TROUGH: [&str; 2] = ["trough_1", "trough_2"];
const EJECT: &str = "trough_eject";
const START: &str = "start";
const LEFT: &str = "left_flipper";
const RIGHT: &str = "right_flipper";

static TROUGH_SWITCHES: LazyLock<Vec<SwitchDefinition>> = LazyLock::new(|| {
  TROUGH
    .iter()
    .map(|name| TroughSystem::switch_definition(name).build())
    .collect()
});
static START_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(START).build());
static LEFT_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(LEFT).build());
static RIGHT_SWITCH: LazyLock<SwitchDefinition> =
  LazyLock::new(|| SwitchDefinition::new(RIGHT).build());
static EJECT_COIL: LazyLock<DriverDefinition> =
  LazyLock::new(|| TroughSystem::eject_coil_definition(EJECT).build());

static HIGH_SCORES: LazyLock<Leaderboard<u64>> = LazyLock::new(|| {
  Leaderboard::highest("High Scores", 2)
    .key("high_scores")
    .rank_titles(["Grand Champion"])
});

fn start(name: &str) -> Scenario {
  let dir = std::env::temp_dir().join(format!(
    "frontbox-leaderboards-flow-{}-{name}",
    std::process::id()
  ));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();

  let mut board = IoBoard::custom(16, 8, DriverVoltage::High)
    .wire_switch(4, &START_SWITCH)
    .wire_switch(5, &LEFT_SWITCH)
    .wire_switch(6, &RIGHT_SWITCH)
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
    .system(StandardBallManager::builder(TROUGH, EJECT).build())
    .system(CompetitiveGame::new(2, vec![], SwitchQ::name(START)))
    .system(LayeredLightManager::new())
    .system(CreditsSystem::new(SwitchQ::name(START)))
    .system(
      LeaderboardsSystem::new(dir.join("leaderboards.toml"))
        .final_scores(&HIGH_SCORES)
        .initials_entry(
          SwitchQ::name(LEFT),
          SwitchQ::name(RIGHT),
          SwitchQ::name(START),
        ),
    )
    .routine::<GameEnding>(routine![LeaderboardsSystem]);
  Scenario::new(app).closed(TROUGH).start()
}

fn entering(s: &mut Scenario) -> Option<u8> {
  s.act(|ctx| {
    let leaderboards = ctx.systems.expect::<LeaderboardsSystem>();
    leaderboards.entry().map(|entry| entry.player())
  })
}

fn names(s: &mut Scenario) -> Vec<String> {
  s.act(|ctx| {
    let leaderboards = ctx.systems.expect::<LeaderboardsSystem>();
    let entries = leaderboards.entries(&HIGH_SCORES);
    entries.into_iter().map(|entry| entry.name).collect()
  })
}

#[test]
fn the_game_ends_once_each_player_has_entered_initials() {
  let mut s = start("entry");
  let entered = s.record(|e: &InitialsEntered| (e.player, e.initials.clone()));
  s.hit_switch(START);
  s.hit_switch(START);
  s.act(|ctx| ctx.quit_game(GameEndReason::Quit));

  // the game waits on player 1
  assert!(s.act(|ctx| ctx.states.is(GamePhase::Ending)));
  assert_eq!(entering(&mut s), Some(0));
  s.hit_switch(RIGHT);
  s.hit_switch(START);
  // left of `A` is back, then a space
  s.hit_switch(LEFT);
  s.hit_switch(LEFT);
  s.hit_switch(START);
  s.hit_switch(START);
  assert_eq!(entered.take(), [(0, "B A".to_string())]);

  // then on player 2, who holds the button down
  assert_eq!(entering(&mut s), Some(1));
  s.close_switch(RIGHT);
  s.advance(Duration::from_millis(700));
  s.open_switch(RIGHT);
  let held = s.act(|ctx| {
    let leaderboards = ctx.systems.expect::<LeaderboardsSystem>();
    leaderboards.entry().unwrap().letters()[0]
  });
  assert!(held > Some('B'), "the letter kept changing: {held:?}");
  s.advance(Duration::from_secs(1));
  for _ in 0..INITIALS_LENGTH {
    s.hit_switch(START);
  }

  assert_eq!(entering(&mut s), None);
  assert!(s.act(|ctx| ctx.states.is(GamePhase::None)));
  assert_eq!(names(&mut s).len(), 2);
  assert_eq!(names(&mut s)[0], "B A");
}

#[test]
fn a_player_who_walks_away_gets_the_initials_as_they_stand() {
  let mut s = start("timeout");
  s.hit_switch(START);
  s.act(|ctx| ctx.quit_game(GameEndReason::Quit));
  assert_eq!(entering(&mut s), Some(0));
  s.hit_switch(RIGHT);
  s.hit_switch(START);

  s.advance(Duration::from_secs(59));
  assert_eq!(entering(&mut s), Some(0));
  s.advance(Duration::from_secs(2));
  assert_eq!(entering(&mut s), None);
  assert!(s.act(|ctx| ctx.states.is(GamePhase::None)));
  assert_eq!(names(&mut s), ["BA"]);
}

#[test]
fn no_initials_are_taken_after_a_slam_tilt() {
  let mut s = start("slam");
  s.hit_switch(START);
  s.act(|ctx| ctx.quit_game(GameEndReason::SlamTilted));
  assert_eq!(entering(&mut s), None);
  assert!(s.act(|ctx| ctx.states.is(GamePhase::None)));
  assert!(names(&mut s).is_empty());
}
