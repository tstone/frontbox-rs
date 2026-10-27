use std::sync::LazyLock;

use frontbox::prelude::*;

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

pub static BALL_SEARCH_DELAY: LazyLock<Config<Duration>> = LazyLock::new(|| {
  Config::new(
    "Ball Search Delay",
    "How long without a switch hit before searching for a stuck ball",
    Duration::from_secs(15),
    Ranges::seconds(5, 60),
  )
  .key("ball_search_delay")
});

pub static TROUGH_EJECT_RETRIES: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "Trough Eject Retries",
    "How many times a failed trough eject is retried before giving up",
    4,
    Ranges::u8(0, 10),
  )
  .key("trough_eject_retries")
});
