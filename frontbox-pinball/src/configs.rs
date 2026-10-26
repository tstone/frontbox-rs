use std::sync::LazyLock;

use frontbox::prelude::*;

pub static TURN_COUNT: LazyLock<Config<u8>> = LazyLock::new(|| {
  Config::new(
    "turn_count",
    "How many turns (balls) players have per game",
    3,
    Ranges::u8(1, 5),
  )
});
