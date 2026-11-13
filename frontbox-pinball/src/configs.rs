//! Operator settings used by this crate's systems. They work without any setup, using their defaults until changed.
//! To let the operator change them, add them to a menu:
//!
//! ```rust,ignore
//! MenuSection::new("Game").configs(vec![
//!   configs::TURN_COUNT.display(),
//!   configs::BALL_SAVE_TIME.display(),
//! ])
//! ```
//!
//! See [`frontbox_core::operator_config`] for how configs work.

use std::sync::LazyLock;

use frontbox_core::prelude::*;

pub static TURN_COUNT: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Turn Count",
    "How many turns (balls) players have per game",
    3,
    Ranges::u8(1, 5),
  )
  .key("turn_count")
});

pub static BALL_SAVE_TIME: LazyLock<Config<Duration>> = LazyLock::new(|| {
  Config::new(
    "Ball Save Time",
    "How long drained balls are saved at the start of each turn (0 for off)",
    Duration::from_secs(8),
    Ranges::seconds(0, 20),
  )
  .key("ball_save_time")
});

pub static MAX_EXTRA_BALLS: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Max Extra Balls",
    "How many extra balls a player can be granted per game (0 for none)",
    5,
    Ranges::u8(0, 10),
  )
  .key("max_extra_balls")
});

pub static TILT_WARNINGS: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Tilt Warnings",
    "How many tilt warnings a player gets before the next swing of the tilt bob tilts",
    2,
    Ranges::u8(0, 5),
  )
  .key("tilt_warnings")
});

pub static TILT_WARNINGS_PER_TURN: LazyLock<Config<bool>> = LazyLock::new(|| {
  Config::new(
    "Tilt Warnings Per Turn",
    "Whether tilt warnings are given back at the start of each turn, rather than once per game",
    true,
    Toggle,
  )
  .key("tilt_warnings_per_turn")
});

pub static BALL_SEARCH_DELAY: LazyLock<Config<Duration>> = LazyLock::new(|| {
  Config::new(
    "Ball Search Delay",
    "How long without a switch hit before searching for a stuck ball",
    Duration::from_secs(15),
    Ranges::seconds(5, 60),
  )
  .key("ball_search_delay")
});

pub static IGNORE_DRAINS_WITH_COIN_DOOR_OPEN: LazyLock<Config<bool>> = LazyLock::new(|| {
  Config::new(
    "Ignore Drains With Door Open",
    "Whether a ball that drains while the coin door is open is put back into play, rather than ending the turn or being saved",
    true,
    Toggle,
  )
  .key("ignore_drains_with_coin_door_open")
});

pub static BALLS_INSTALLED: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Balls Installed",
    "How many balls are in the machine (0 for one per trough switch)",
    0,
    Ranges::u8(0, 10),
  )
  .key("balls_installed")
});

pub static TROUGH_EJECT_RETRIES:LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Trough Eject Retries",
    "How many times a failed trough eject is retried before giving up",
    4,
    Ranges::u8(0, 10),
  )
  .key("trough_eject_retries")
});

pub static AUTOPLUNGER_RETRIES: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Autoplunger Retries",
    "How many times the autoplunger fires again at a ball it didn't launch before giving up",
    3,
    Ranges::u8(0, 10),
  )
  .key("autoplunger_retries")
});
