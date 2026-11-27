use std::marker::PhantomData;
use std::time::Duration;

/// A typed leaderboard: a ranked list of the best values players have earned. See [leaderboards](crate::leaderboards).
#[derive(Clone, Debug)]
pub struct Leaderboard<T> {
  definition: LeaderboardDefinition,
  _type: PhantomData<fn() -> T>,
}

impl<T: LeaderboardValue> Leaderboard<T> {
  /// A board where larger values rank first (points, counts), keeping `capacity` entries
  pub fn highest(label: &'static str, capacity: usize) -> Self {
    Self::new(label, capacity, Direction::Highest)
  }

  /// A board where smaller values rank first (times), keeping `capacity` entries
  pub fn lowest(label: &'static str, capacity: usize) -> Self {
    Self::new(label, capacity, Direction::Lowest)
  }

  fn new(label: &'static str, capacity: usize, direction: Direction) -> Self {
    Self {
      definition: LeaderboardDefinition {
        key: label,
        label,
        direction,
        capacity,
        rank_titles: Vec::new(),
        defaults: Vec::new(),
        type_name: std::any::type_name::<T>(),
        formatter: T::format,
      },
      _type: PhantomData,
    }
  }

  /// Set the board's `key`, which its entries are saved under. Key must be unique but is derived from label by
  /// default. This force-sets it to a specific value for uniqueness.
  pub fn key(mut self, key: &'static str) -> Self {
    self.definition.key = key;
    self
  }

  /// Name the top ranks, best first. Ranks past the last title are numbered from 1: with `["Grand Champion"]`, the
  /// ranks are Grand Champion, #1, #2, and so on.
  pub fn rank_titles(mut self, titles: impl IntoIterator<Item = &'static str>) -> Self {
    self.definition.rank_titles = titles.into_iter().collect();
    self
  }

  /// Entries the board starts with, for players to beat. They aren't saved: they're ranked alongside what players
  /// have earned each time the board is read.
  pub fn defaults(mut self, defaults: impl IntoIterator<Item = (&'static str, T)>) -> Self {
    self.definition.defaults = defaults
      .into_iter()
      .map(|(name, value)| (name, value.to_raw()))
      .collect();
    self
  }

  /// The untyped declaration, for listing alongside boards of other types
  pub fn definition(&self) -> &LeaderboardDefinition {
    &self.definition
  }
}

/// A leaderboard's declaration without its value type, so boards of different types can be listed together
#[derive(Clone, Debug)]
pub struct LeaderboardDefinition {
  pub key: &'static str,
  /// What players see
  pub label: &'static str,
  pub direction: Direction,
  /// How many entries the board keeps
  pub capacity: usize,
  pub(crate) rank_titles: Vec<&'static str>,
  /// Names and raw values, see [`LeaderboardValue::to_raw`]
  pub(crate) defaults: Vec<(&'static str, u64)>,
  /// The Rust type of the board's values, so a `Duration` and a `u64` don't share a key
  pub(crate) type_name: &'static str,
  pub(crate) formatter: fn(u64) -> String,
}

impl LeaderboardDefinition {
  /// What a rank is called, counting from 0 for the best: its title, or its number among the untitled ranks
  pub fn rank_title(&self, rank: usize) -> String {
    match self.rank_titles.get(rank) {
      Some(title) => title.to_string(),
      None => format!("#{}", rank + 1 - self.rank_titles.len()),
    }
  }

  /// Whether a rank (from 0) has a title of its own, rather than a number
  pub fn has_rank_title(&self, rank: usize) -> bool {
    rank < self.rank_titles.len()
  }

  /// What holding a rank (from 0) is called on its own, away from the rest of the board: the rank's title ("Grand
  /// Champion"), the board's name where it keeps a single entry ("Loop Champion"), or the board's name and the rank's
  /// number ("High Scores #1")
  pub fn place_title(&self, rank: usize) -> String {
    if self.has_rank_title(rank) {
      self.rank_title(rank)
    } else if self.capacity == 1 {
      self.label.to_string()
    } else {
      format!("{} {}", self.label, self.rank_title(rank))
    }
  }

  /// How a raw value is shown to players
  pub fn format(&self, value: u64) -> String {
    (self.formatter)(value)
  }

  /// Orders raw values best first
  pub(crate) fn order(&self, a: u64, b: u64) -> std::cmp::Ordering {
    match self.direction {
      Direction::Highest => b.cmp(&a),
      Direction::Lowest => a.cmp(&b),
    }
  }
}

/// Which values rank first on a leaderboard
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
  Highest,
  Lowest,
}

/// A Rust type that can be ranked on a [`Leaderboard`], by mapping itself to the `u64` it's saved and compared as
pub trait LeaderboardValue {
  fn to_raw(&self) -> u64;
  fn from_raw(raw: u64) -> Self;
  /// How a saved value is shown to players
  fn format(raw: u64) -> String;
}

macro_rules! integer_leaderboard_value {
  ($($t:ty),*) => {
    $(
      impl LeaderboardValue for $t {
        fn to_raw(&self) -> u64 {
          *self as u64
        }

        fn from_raw(raw: u64) -> Self {
          raw as $t
        }

        fn format(raw: u64) -> String {
          with_thousands(raw)
        }
      }
    )*
  };
}

integer_leaderboard_value!(u8, u16, u32, u64, usize);

/// Saved as milliseconds, and shown as minutes, seconds and hundredths (`1:05.25`)
impl LeaderboardValue for Duration {
  fn to_raw(&self) -> u64 {
    self.as_millis() as u64
  }

  fn from_raw(raw: u64) -> Self {
    Duration::from_millis(raw)
  }

  fn format(raw: u64) -> String {
    let minutes = raw / 60_000;
    let seconds = raw % 60_000 / 1000;
    let hundredths = raw % 1000 / 10;
    format!("{minutes}:{seconds:02}.{hundredths:02}")
  }
}

/// `1234567` as `1,234,567`
fn with_thousands(value: u64) -> String {
  let digits = value.to_string();
  let mut out = String::with_capacity(digits.len() + digits.len() / 3);
  for (i, digit) in digits.chars().enumerate() {
    if i > 0 && (digits.len() - i).is_multiple_of(3) {
      out.push(',');
    }
    out.push(digit);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ranks_past_the_titles_are_numbered_from_one() {
    let board = Leaderboard::<u64>::highest("High Scores", 5).rank_titles(["Grand Champion"]);
    let definition = board.definition();

    assert_eq!(definition.rank_title(0), "Grand Champion");
    assert_eq!(definition.rank_title(1), "#1");
    assert_eq!(definition.rank_title(4), "#4");
  }

  #[test]
  fn values_are_formatted_by_their_type() {
    assert_eq!(<u64 as LeaderboardValue>::format(1_234_567), "1,234,567");
    assert_eq!(<u32 as LeaderboardValue>::format(12), "12");
    assert_eq!(<Duration as LeaderboardValue>::format(65_250), "1:05.25");
  }
}
