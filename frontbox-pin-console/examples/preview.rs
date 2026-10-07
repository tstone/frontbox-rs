//! Runs a virtual machine with a scripted fake game, so the console has hardware, systems and a
//! live stream of events to show without any real hardware attached.
//!
//! `cargo run --example preview`, then open http://localhost:3000 (or run `npm run dev` in `web/`).
//! Set `CONSOLE_PORT` to serve somewhere other than 3000, and `PLAYFIELD_IMAGE` / `BACKBOX_IMAGE` to lay art over
//! those planes. `CONSOLE_STRESS` fires every driver every tick, to load the console with traffic.

use frontbox::animation::*;
use frontbox::prelude::*;
use frontbox_pin_console::{WebTracer, console_plane};
use frontbox_turn_based::{GameEnded, GameStarted, PlayerTurnBeginning};
use std::io::Write;
use std::time::Duration;

use crate::hardware::*;

/// Surfaces of the machine, in inches from the cabinet's back-left-bottom corner (x right, y toward the front, z up)
mod planes {
  use super::*;
  use std::f32::consts::FRAC_1_SQRT_2;
  use std::sync::LazyLock;

  // rotations as (x, y, z, w) quaternions, so the planes can be plain statics
  /// 90° about x: the plane stands upright, local y pointing up
  const UPRIGHT: Quat = Quat::from_xyzw(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2);
  /// Along a side wall: local x toward the front, local y up
  const ALONG_SIDE: Quat = Quat::from_xyzw(0.5, 0.5, 0.5, 0.5);

  pub static PLAYFIELD: LazyLock<ReferencePlane> = LazyLock::new(|| {
    with_image(
      ReferencePlane::new("Playfield")
        .origin(Vec3::new(1.0, 3.25, 12.0))
        .extent(Vec2::new(20.25, 45.0)),
      "PLAYFIELD_IMAGE",
    )
  });

  pub static BACKBOX: LazyLock<ReferencePlane> = LazyLock::new(|| {
    with_image(
      ReferencePlane::new("Backbox")
        .parent(&PLAYFIELD)
        .origin(Vec3::new(-1.5, -3.25, 22.0))
        .extent(Vec2::new(23.25, 30.0))
        .rotation(UPRIGHT),
      "BACKBOX_IMAGE",
    )
  });

  pub static CABINET_LEFT: ReferencePlane = ReferencePlane::new("Cabinet left")
    .origin(Vec3::new(0.0, 0.0, 12.0))
    .extent(Vec2::new(50.0, 6.0))
    .rotation(ALONG_SIDE)
    .build();

  pub static CABINET_RIGHT: ReferencePlane = ReferencePlane::new("Cabinet right")
    .origin(Vec3::new(22.25, 0.0, 12.0))
    .extent(Vec2::new(50.0, 6.0))
    .rotation(ALONG_SIDE)
    .build();

  /// PLAYFIELD_IMAGE / BACKBOX_IMAGE: optional art to lay over those planes
  fn with_image(plane: ReferencePlaneBuilder, env: &str) -> ReferencePlane {
    match std::env::var(env) {
      Ok(path) => plane.image(path).build(),
      Err(_) => plane.build(),
    }
  }

  /// Evenly spaced points along a plane, at height `y`
  pub fn row(plane: &'static ReferencePlane, count: u16, y: f32) -> Vec<Vec3> {
    let spacing = plane.extent.x / count as f32;
    (0..count)
      .map(|i| Vec2::new(spacing * (i as f32 + 0.5), y).relative_to(plane))
      .collect()
  }
}

mod hardware {
  use super::planes::*;
  use super::*;

  hardware_defs! {
    pub START_BUTTON: SwitchDefinition = SwitchDefinition::new("start_button")
      // on the cabinet front, which isn't a plane here; a location can always be given directly
      .location(Vec3::new(17.0, 50.5, 9.0));
    pub LEFT_FLIPPER_BUTTON: SwitchDefinition = SwitchDefinition::new("left_flipper_button")
      .location(Vec3::new(0.0, 44.0, 9.0));
    pub TROUGH_1: SwitchDefinition = SwitchDefinition::new("trough_1")
      .location(Vec2::new(10.1, 44.2).relative_to(&PLAYFIELD));
    pub LEFT_FLIPPER_EOS: SwitchDefinition = SwitchDefinition::new("left_flipper_eos")
      .location(Vec2::new(6.4, 41.6).relative_to(&PLAYFIELD));
    pub LEFT_SLING: SwitchDefinition = SwitchDefinition::new("left_sling")
      .location(Vec2::new(4.6, 33.5).relative_to(&PLAYFIELD))
      .debounce_close(Duration::from_millis(2))
      .debounce_open(Duration::from_millis(10));

    // automatic: fired by the hardware from a switch
    pub LEFT_FLIPPER: DriverDefinition = DriverDefinition::new("left_flipper")
      .location(Vec2::new(7.0, 41.0).relative_to(&PLAYFIELD))
      .mode(DriverMode::flipper_main_direct(LEFT_FLIPPER_BUTTON.name, LEFT_FLIPPER_EOS.name).build());
    pub LEFT_SLING_COIL: DriverDefinition = DriverDefinition::new("left_sling_coil")
      .location(Vec2::new(5.4, 32.4).relative_to(&PLAYFIELD))
      .mode(DriverMode::pulse().trigger_mode(DriverTriggerMode::Switch(LEFT_SLING.name)).build());

    // commanded by software
    pub TROUGH_EJECT: DriverDefinition = DriverDefinition::new("trough_eject")
      .location(Vec2::new(10.1, 43.0).relative_to(&PLAYFIELD));
    pub SCOOP_EJECT: DriverDefinition = DriverDefinition::new("scoop_eject")
      .location(Vec2::new(2.5, 10.9).relative_to(&PLAYFIELD));

    pub SHOOT_AGAIN: LedDefinition = LedDefinition::single("shoot_again")
      .location(Vec2::new(10.1, 39.4).relative_to(&PLAYFIELD));
    pub LANE_1: LedDefinition = LedDefinition::single("lane_1")
      // wired GRB, so the console has to undo the channel order to show it red
      .channels(LedChannels::GRB)
      .location(Vec2::new(8.0, 5.0).relative_to(&PLAYFIELD));
    pub LANE_2: LedDefinition = LedDefinition::single("lane_2")
      .location(Vec2::new(12.0, 5.0).relative_to(&PLAYFIELD));
    pub SCOOP_ARROW: LedDefinition = LedDefinition::single("scoop_arrow")
      .location(Vec2::new(3.5, 13.0).relative_to(&PLAYFIELD));
    pub INSERTS: LedDefinition = LedDefinition::multi("inserts")
      .locations((0..5).map(|i| Vec2::new(10.1, 25.5 + i as f32 * 2.1).relative_to(&PLAYFIELD)));
    pub BACKBOX_GI: LedDefinition = LedDefinition::multi("backbox_gi").locations(row(&BACKBOX, 8, 27.5));
    pub CABINET_LEFT_STRIP: LedDefinition = LedDefinition::multi("cabinet_left_strip")
      .locations(row(&CABINET_LEFT, 14, 3.0));
    pub CABINET_RIGHT_STRIP: LedDefinition = LedDefinition::multi("cabinet_right_strip")
      .locations(row(&CABINET_RIGHT, 14, 3.0));
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
    ExpBoard::neuron()
      .wire_led_port(1, LedPort::ws2812().leds(vec![&SHOOT_AGAIN, &INSERTS]))
      .wire_led_port(2, LedPort::ws2812().leds(vec![&CABINET_LEFT_STRIP]))
      .wire_led_port(3, LedPort::ws2812().leds(vec![&CABINET_RIGHT_STRIP])),
    ExpBoard::fp_exp0061(JumperState::Open, JumperState::Open)
      .wire_led_port(
        1,
        LedPort::ws2812().leds(vec![&LANE_1, &LANE_2, &SCOOP_ARROW]),
      )
      .wire_led_port(2, LedPort::ws2812().leds(vec![&BACKBOX_GI])),
  ]);

  let mut tracer = WebTracer::new()
    .plane(console_plane!(planes::PLAYFIELD))
    .plane(console_plane!(planes::BACKBOX))
    .plane(console_plane!(planes::CABINET_LEFT))
    .plane(console_plane!(planes::CABINET_RIGHT));
  // CONSOLE_PORT lets the preview run alongside a game that already has the console on :3000
  if let Ok(port) = std::env::var("CONSOLE_PORT") {
    tracer = tracer.port(port.parse().expect("CONSOLE_PORT must be a port number"));
  }

  App::new(BootConfig {
    io_network,
    exp_network,
    platform: Platform::Virtual,
    ..Default::default()
  })
  .configure(|app| {
    app
      .tracer(tracer)
      .system(LedSystem::new())
      .system(LightShow::new())
      .system(Attract)
      .system(ScoreKeeper);
    // CONSOLE_STRESS: fire every driver every tick, to see how the console holds up under heavy traffic. The fake
    // game is left out, like a machine sitting in attract mode, since each game start clears the console's log.
    if std::env::var("CONSOLE_STRESS").is_ok() {
      app.system(DriverStress);
    } else {
      app.system(FakeGame::default());
    }
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

/// Gives the console LED colors to show: fixed colors on the playfield, and a pulse on the shoot again insert, cabinet
/// strips and backbox
struct LightShow {
  pulse: Box<dyn Animation<Duration, Rgba<u8>>>,
}

impl LightShow {
  fn new() -> Self {
    Self {
      pulse: Tween::boxed(
        Duration::from_millis(900),
        Curve::Sinusoid,
        vec![Rgba::black(), Rgba::purple()],
        Cycle::Forever,
      ),
    }
  }
}

impl System for LightShow {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.declare_leds(&LANE_1.q(), ColorSequence::solid(Rgba::red()));
    ctx.declare_leds(&LANE_2.q(), ColorSequence::solid(Rgba::blue()));
    ctx.declare_leds(&INSERTS.q(), ColorSequence::fade(Rgba::yellow(), Rgba::red()));
    ctx.declare_leds(&CABINET_LEFT_STRIP.q(), ColorSequence::fade(Rgba::blue(), Rgba::purple()));
    ctx.declare_leds(&CABINET_RIGHT_STRIP.q(), ColorSequence::fade(Rgba::purple(), Rgba::blue()));
    ctx.declare_leds(&BACKBOX_GI.q(), ColorSequence::solid(Rgba::white()));
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    self.pulse.accumulate(delta);
    let color = self.pulse.sample();
    ctx.declare_leds(&SHOOT_AGAIN.q(), ColorSequence::solid(color));
    // enough LEDs changing every tick that one LED frame spans several batches, like a real machine
    ctx.declare_leds(&CABINET_LEFT_STRIP.q().at_z(1), ColorSequence::solid(color));
    ctx.declare_leds(&CABINET_RIGHT_STRIP.q().at_z(1), ColorSequence::solid(color));
    ctx.declare_leds(&BACKBOX_GI.q().at_z(1), ColorSequence::solid(color));
  }
}

/// Taps every driver every tick, for `CONSOLE_STRESS`
struct DriverStress;

impl System for DriverStress {
  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    for driver in [&LEFT_FLIPPER, &LEFT_SLING_COIL, &TROUGH_EJECT, &SCOOP_EJECT] {
      ctx.activate_driver(driver.name, ActivationMode::Tap);
    }
  }
}
