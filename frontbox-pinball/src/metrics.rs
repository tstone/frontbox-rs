//! Metrics recorded by this crate's systems, without any setup. See [`frontbox_core::metrics`] for how metrics work.
//!
//! Some audits need no metric of their own: game durations and player counts are kept for every finished game, so
//! they're queries over its games.

use std::sync::LazyLock;

use frontbox_core::prelude::*;

pub static GAMES_STARTED: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new(
    "Games Started",
    "Games started, including ones cut off before they ended",
  )
  .key("games_started")
  .category("Games")
});

pub static TURNS_PLAYED: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Turns Played", "Turns (balls) started")
    .key("turns_played")
    .category("Games")
});

pub static BALL_TIME: LazyLock<Sample<Duration>> = LazyLock::new(|| {
  Sample::new(
    "Ball Time",
    "Time from a turn becoming active to its last ball leaving play",
  )
  .key("ball_time")
  .category("Games")
});

pub static FINAL_SCORE: LazyLock<Sample<u32>> = LazyLock::new(|| {
  Sample::new("Final Score", "Each player's score at the end of a game")
    .key("final_score")
    .category("Games")
});

pub static GAMES_QUIT: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Games Quit", "Games ended early with the quit game combo")
    .key("games_quit")
    .category("Games")
});

pub static TILT_WARNINGS: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Tilt Warnings", "Tilt warnings given")
    .key("tilt_warnings")
    .category("Games")
});

pub static TILTS: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Tilts", "Turns lost to a tilt")
    .key("tilts")
    .category("Games")
});

pub static SLAM_TILTS: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Slam Tilts", "Slam tilts, during a game or not")
    .key("slam_tilts")
    .category("Games")
});

pub static BALLS_SERVED: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new(
    "Balls Served",
    "Balls ejected from the trough, once verified",
  )
  .key("balls_served")
  .category("Ball Management")
});

pub static BALLS_DRAINED: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Balls Drained", "Balls that left play, saved or not")
    .key("balls_drained")
    .category("Ball Management")
});

pub static BALL_SAVES: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Ball Saves", "Drained balls returned by ball save")
    .key("ball_saves")
    .category("Ball Management")
});

pub static TROUGH_EJECT_FAILURES: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new(
    "Trough Eject Failures",
    "Trough ejects given up on after every retry",
  )
  .key("trough_eject_failures")
  .category("Ball Management")
});

pub static BALL_SEARCHES: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Ball Searches", "Ball searches started")
    .key("ball_searches")
    .category("Ball Management")
});

pub static EXTRA_BALLS: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Extra Balls", "Extra balls used for another turn")
    .key("extra_balls")
    .category("Games")
});
