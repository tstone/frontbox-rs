use std::sync::LazyLock;

use frontbox::prelude::*;

pub static TURN_COUNT: LazyLock<ConfigValue<u8, Range<u8>>> = LazyLock::new(|| {
  ConfigValue::new(
    "turn_count",
    "How many turns (balls) players have per game",
    3,
    Ranges::u8(1, 5),
  )
});
