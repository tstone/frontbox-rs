//! Runs a virtual machine with a scripted fake game, so the console has hardware, systems and a
//! live stream of events to show without any real hardware attached.
//!
//! `cargo run --example preview`, then open http://localhost:3000 (or run `npm run dev` in `web/`).
//! Set `CONSOLE_PORT` to serve somewhere other than 3000.

use frontbox::prelude::*;
use frontbox_pin_console::WebTracer;
use frontbox_turn_based::{GameEnded, GameStarted, PlayerTurnBeginning};
use std::io::Write;
use std::time::Duration;

use crate::hardware::*;

mod hardware {
  use super::*;

  hardware_defs! {
    pub START_BUTTON: SwitchDefinition = SwitchDefinition::new("start_button");
    pub LEFT_FLIPPER_BUTTON: SwitchDefinition = SwitchDefinition::new("left_flipper_button");
    pub TROUGH_1: SwitchDefinition = SwitchDefinition::new("trough_1");
    pub LEFT_FLIPPER_EOS: SwitchDefinition = SwitchDefinition::new("left_flipper_eos");
    pub LEFT_SLING: SwitchDefinition = SwitchDefinition::new("left_sling")
      .debounce_close(Duration::from_millis(2))
      .debounce_open(Duration::from_millis(10));

    // automatic: fired by the hardware from a switch
    pub LEFT_FLIPPER: DriverDefinition = DriverDefinition::new("left_flipper")
      .mode(DriverMode::flipper_main_direct(LEFT_FLIPPER_BUTTON.name, LEFT_FLIPPER_EOS.name).build());
    pub LEFT_SLING_COIL: DriverDefinition = DriverDefinition::new("left_sling_coil")
      .mode(DriverMode::pulse().trigger_mode(DriverTriggerMode::Switch(LEFT_SLING.name)).build());

    // commanded by software
    pub TROUGH_EJECT: DriverDefinition = DriverDefinition::new("trough_eject");
    pub SCOOP_EJECT: DriverDefinition = DriverDefinition::new("scoop_eject");

    pub SHOOT_AGAIN: LedDefinition = LedDefinition::single("shoot_again");
    pub LANE_1: LedDefinition = LedDefinition::single("lane_1");
    pub LANE_2: LedDefinition = LedDefinition::single("lane_2");
    pub SCOOP_ARROW: LedDefinition = LedDefinition::single("scoop_arrow");
  }
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let io_network = IoNetwork::new(vec![
    IoBoards::io_3208()
      .wire_switch(0, &START_BUTTON)
      .wire_switch(1, &LEFT_FLIPPER_BUTTON)
      .wire_switch(2, &TROUGH_1)
      .wire_switch(3, &LEFT_FLIPPER_EOS)
      .wire_switch(4, &LEFT_SLING)
      .wire_driver(0, &LEFT_FLIPPER)
      .wire_driver(1, &LEFT_SLING_COIL)
      .wire_driver(2, &TROUGH_EJECT)
      .wire_driver(3, &SCOOP_EJECT),
  ]);

  let exp_network = ExpNetwork::new(vec![
    ExpBoard::neuron().wire_led_port(1, LedPort::ws2812().leds(vec![&SHOOT_AGAIN])),
    ExpBoard::fp_exp0061(JumperState::Open, JumperState::Open).wire_led_port(
      1,
      LedPort::ws2812().leds(vec![&LANE_1, &LANE_2, &SCOOP_ARROW]),
    ),
  ]);

  // CONSOLE_PORT lets the preview run alongside a game that already has the console on :3000
  let tracer = match std::env::var("CONSOLE_PORT") {
    Ok(port) => WebTracer::at_addr(([0, 0, 0, 0], port.parse().expect("CONSOLE_PORT must be a port number")).into()),
    Err(_) => WebTracer::new(),
  };

  App::new(BootConfig {
    io_network,
    exp_network,
    platform: Platform::Virtual,
    ..Default::default()
  })
  .configure(|app| {
    app
      .tracer(tracer)
      .system(FakeGame::default())
      .system(Attract)
      .system(ScoreKeeper);
  })
  .run()
  .await;
}

const PLAYERS: u8 = 2;
const TURNS: u8 = 3;
/// Beats between each player's turn starting
const BEATS_PER_TURN: u32 = 5;

/// Plays through a game on a loop: start, a few turns per player with coils firing, then end
#[derive(Default)]
struct FakeGame {
  beat: u32,
}

#[derive(serde::Serialize, Event)]
struct Beat;

impl System for FakeGame {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.cue(Beat, Cue::Forever(Duration::from_millis(700)));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if !event.is::<Beat>() {
      return;
    }

    let turns_total = (PLAYERS as u32) * (TURNS as u32);
    let game_length = 1 + turns_total * BEATS_PER_TURN;
    let beat = self.beat % (game_length + 3);
    self.beat += 1;

    if beat == 0 {
      ctx.emit(GameStarted);
      // like a turn-based game, each player gets a group that's only active during their turn
      for group in PLAYER_GROUPS {
        ctx.spawn_system_group(group, vec![PlayerModes.into(), BallSave.into()], false);
      }
    } else if beat < game_length && (beat - 1) % BEATS_PER_TURN == 0 {
      let turn_index = (beat - 1) / BEATS_PER_TURN;
      let player = (turn_index % PLAYERS as u32) as u8;
      let turn = (turn_index / PLAYERS as u32) as u8 + 1;
      for (index, group) in PLAYER_GROUPS.iter().enumerate() {
        if index == player as usize {
          ctx.activate_system_group(group);
        } else {
          ctx.deactivate_system_group(group);
        }
      }
      ctx.emit(PlayerTurnBeginning::new(player, turn));
      ctx.activate_driver(TROUGH_EJECT.name, ActivationMode::Tap);
    } else if beat < game_length {
      ctx.activate_driver(SCOOP_EJECT.name, ActivationMode::Tap);
    } else if beat == game_length {
      ctx.emit(GameEnded {
        scores: vec![("Player 1", 125_000), ("Player 2", 98_500)],
      });
      for group in PLAYER_GROUPS {
        ctx.despawn_system_group(group);
      }
    }
  }
}

const PLAYER_GROUPS: [&str; PLAYERS as usize] = ["player_1", "player_2"];

/// Placeholders so the systems tree has more than one entry
struct Attract;
impl System for Attract {}

struct ScoreKeeper;
impl System for ScoreKeeper {}

#[derive(Clone)]
struct PlayerModes;
impl System for PlayerModes {}

#[derive(Clone)]
struct BallSave;
impl System for BallSave {}
