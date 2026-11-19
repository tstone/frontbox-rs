//! Coins, credits and the start button, on virtual hardware

use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use frontbox_core::machine::DriverTrigger;
use frontbox_core::prelude::*;
use frontbox_core::testing::Scenario;
use frontbox_lighting::LayeredLightManager;
use frontbox_pinball::*;

// The machine: a two-ball trough, a start button with a lamp, and two coin slots. Its game ends with a game-over
// display that holds `GameEnding` until it's told to let go, so a test can press start while a game is ending.

const TROUGH: [&str; 2] = ["trough_1", "trough_2"];
const EJECT: &str = "trough_eject";
const START: &str = "start";
const START_LAMP: &str = "start_lamp";
const COIN_LEFT: &str = "coin_left";
const COIN_RIGHT: &str = "coin_right";

static TROUGH_SWITCHES: LazyLock<Vec<SwitchDefinition>> = LazyLock::new(|| {
  TROUGH
    .iter()
    .map(|name| TroughSystem::switch_definition(name).build())
    .collect()
});
static START_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(START).build());
static COIN_LEFT_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(COIN_LEFT).build());
static COIN_RIGHT_SWITCH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new(COIN_RIGHT).build());
static EJECT_COIL: LazyLock<DriverDefinition> = LazyLock::new(|| TroughSystem::eject_coil_definition(EJECT).build());
static START_LAMP_DRIVER: LazyLock<DriverDefinition> = LazyLock::new(|| DriverDefinition::light(START_LAMP).build());

fn app(max_players: u8, credits: CreditsSystem) -> App {
  let mut board = IoBoard::custom(16, 8, DriverVoltage::High)
    .wire_switch(4, &START_SWITCH)
    .wire_switch(5, &COIN_LEFT_SWITCH)
    .wire_switch(6, &COIN_RIGHT_SWITCH)
    .wire_driver(0, &EJECT_COIL)
    .wire_driver(1, &START_LAMP_DRIVER);
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
    .system(CompetitiveGame::new(max_players, vec![], SwitchQ::name(START)))
    .system(LayeredLightManager::new())
    .system(credits)
    .system(GameOverDisplay)
    .routine::<GameEnding>(routine![GameOverDisplay]);
  app
}

/// Holds `GameEnding` until `GameOverDone`, as a display waiting out its animation would
#[derive(Clone)]
struct GameOverDisplay;

#[derive(serde::Serialize, Event)]
struct GameOverDone;

impl System for GameOverDisplay {
  /// Starts the animation; the step completes on `GameOverDone`
  fn on_routine(&mut self, _routine: &dyn Event, _ctx: &SystemContext) {}

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<GameOverDone>() {
      ctx.routines.complete_step::<GameEnding>();
    }
  }
}

/// Starts a game and quits it, leaving it held in `GameEnding`, where no player can be added
fn end_game(s: &mut Scenario) {
  s.hit_switch(START);
  s.act(|ctx| ctx.quit_game(GameEndReason::Quit));
  assert!(s.act(|ctx| ctx.states.is(GamePhase::Ending)));
}

fn credits_system() -> CreditsSystem {
  CreditsSystem::new(SwitchQ::name(START))
    .coin_switches(SwitchQ::names([COIN_LEFT, COIN_RIGHT]))
    .start_lamp(LightQ::name(START_LAMP))
}

fn start(max_players: u8) -> Scenario {
  start_with(max_players, credits_system())
}

fn start_with(max_players: u8, credits: CreditsSystem) -> Scenario {
  Scenario::new(app(max_players, credits)).closed(TROUGH).start()
}

/// Free play off, as on a machine taking coins
fn coin_op(max_players: u8) -> Scenario {
  let mut s = start(max_players);
  s.act(|ctx| ctx.config.set(&configs::FREE_PLAY, false));
  s
}

fn credits(s: &mut Scenario) -> u8 {
  s.act(|ctx| ctx.systems.with::<CreditsSystem, _>(|credits| credits.credits()).unwrap())
}

/// Whether the start lamp lights at any point over a few flashes
fn lamp_lights(s: &mut Scenario) -> bool {
  let mut lit = false;
  for _ in 0..10 {
    s.advance(Duration::from_millis(50));
    lit |= s.hardware().driver(START_LAMP).unwrap().trigger == DriverTrigger::On;
  }
  lit
}

/// A fresh, empty directory per test
fn test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir().join(format!("frontbox-credits-{}-{name}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

// Free play

#[test]
fn free_play_starts_a_game_without_spending_a_credit() {
  let mut s = start(1);
  let added = s.record(|_: &PlayerAdded| ());

  s.hit_switch(COIN_LEFT);
  s.hit_switch(START);
  assert_eq!(added.take().len(), 1);
  assert!(s.act(|ctx| !ctx.states.is(GamePhase::None)));
  assert_eq!(credits(&mut s), 1);
}

#[test]
fn free_play_refuses_a_press_while_a_game_cant_be_started() {
  let mut s = start(1);
  end_game(&mut s);
  let added = s.record(|_: &PlayerAdded| ());
  let refused = s.record(|e: &StartRefused| e.reason);

  s.hit_switch(START);
  assert!(added.take().is_empty());
  assert_eq!(refused.take(), [StartRefusedReason::GameEnding]);

  // once the game has ended, the next press starts one
  s.emit(GameOverDone);
  s.hit_switch(START);
  assert_eq!(added.take().len(), 1);
}

// Credits

#[test]
fn a_credit_starts_a_game() {
  let mut s = coin_op(1);
  let added = s.record(|_: &PlayerAdded| ());

  s.hit_switch(COIN_LEFT);
  s.hit_switch(START);
  assert_eq!(added.take().len(), 1);
  assert_eq!(credits(&mut s), 0);
}

#[test]
fn with_no_credit_a_press_is_refused() {
  let mut s = coin_op(1);
  let added = s.record(|_: &PlayerAdded| ());
  let refused = s.record(|e: &StartRefused| e.reason);

  s.hit_switch(START);
  assert!(added.take().is_empty());
  assert_eq!(refused.take(), [StartRefusedReason::NoCredits]);
  assert!(s.act(|ctx| ctx.states.is(GamePhase::None)));
}

#[test]
fn a_press_while_a_game_cant_be_started_spends_no_credit() {
  let mut s = coin_op(1);
  for _ in 0..3 {
    s.hit_switch(COIN_LEFT);
  }
  end_game(&mut s);
  let added = s.record(|_: &PlayerAdded| ());

  s.hit_switch(START);
  assert!(added.take().is_empty());
  assert_eq!(credits(&mut s), 2, "only the game that was played spent one");
}

#[test]
fn each_player_added_spends_a_credit() {
  let mut s = coin_op(2);
  let added = s.record(|_: &PlayerAdded| ());
  let changes = s.record(|e: &CreditsChanged| e.credits);

  s.hit_switch(COIN_LEFT);
  s.hit_switch(COIN_RIGHT);
  s.hit_switch(COIN_LEFT);
  assert_eq!(changes.take(), [1, 2, 3]);

  s.hit_switch(START);
  s.hit_switch(START);
  assert_eq!(added.take().len(), 2);
  assert_eq!(changes.take(), [2, 1]);
  assert_eq!(credits(&mut s), 1);
}

#[test]
fn a_press_once_the_game_is_full_spends_no_credit() {
  let mut s = coin_op(1);
  s.hit_switch(COIN_LEFT);
  s.hit_switch(COIN_LEFT);
  let refused = s.record(|e: &StartRefused| e.reason);

  s.hit_switch(START);
  s.hit_switch(START);
  assert_eq!(credits(&mut s), 1);
  assert_eq!(refused.take(), [StartRefusedReason::GameInProgress]);
}

// Coins

#[test]
fn coins_add_up_to_a_credit() {
  let mut s = coin_op(1);
  s.act(|ctx| ctx.config.set(&configs::COINS_PER_CREDIT, 2));
  let changes = s.record(|e: &CreditsChanged| (e.credits, e.coins));

  s.hit_switch(COIN_LEFT);
  s.hit_switch(COIN_RIGHT);
  s.hit_switch(COIN_LEFT);
  assert_eq!(changes.take(), [(0, 1), (1, 0), (1, 1)]);
}

#[test]
fn credits_stop_at_the_limit() {
  let mut s = coin_op(1);
  s.act(|ctx| ctx.config.set(&configs::MAX_CREDITS, 2));

  for _ in 0..4 {
    s.hit_switch(COIN_LEFT);
  }
  assert_eq!(credits(&mut s), 2);

  s.act(|ctx| ctx.systems.with::<CreditsSystem, _>(|credits| credits.add_credits(5, ctx.into())));
  assert_eq!(credits(&mut s), 2);
}

#[test]
fn credits_can_be_added_up_to_the_limit() {
  let mut s = coin_op(1);
  s.act(|ctx| ctx.config.set(&configs::MAX_CREDITS, 3));
  let changes = s.record(|e: &CreditsChanged| e.credits);

  s.act(|ctx| ctx.systems.with::<CreditsSystem, _>(|credits| credits.add_credits(2, ctx.into())));
  s.act(|ctx| ctx.systems.with::<CreditsSystem, _>(|credits| credits.add_credits(2, ctx.into())));
  assert_eq!(changes.take(), [2, 3]);
}

// Saving

#[test]
fn credits_are_kept_across_power_cycles() {
  let path = test_dir("kept").join("credits.toml");

  let mut s = start_with(1, credits_system().save_to(&path));
  s.hit_switch(COIN_LEFT);
  s.hit_switch(COIN_LEFT);
  drop(s);

  assert_eq!(credits_system().save_to(&path).credits(), 2);
}

// The start lamp

#[test]
fn the_start_lamp_flashes_on_free_play() {
  let mut s = start(1);
  assert!(lamp_lights(&mut s));
}

#[test]
fn the_start_lamp_is_dark_without_a_credit() {
  let mut s = coin_op(1);
  assert!(!lamp_lights(&mut s));

  s.hit_switch(COIN_LEFT);
  assert!(lamp_lights(&mut s), "lit once there's a credit");
}

#[test]
fn the_start_lamp_is_dark_once_no_player_can_be_added() {
  let mut s = start(1);
  s.hit_switch(START);
  assert!(!lamp_lights(&mut s));
}
