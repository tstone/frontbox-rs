#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cycle {
  /// Run once
  #[default]
  Once,
  /// Loop exactly n times
  Times(u32),
  /// Loop continually
  Forever,
}

impl Cycle {
  /// Where `time` falls in a cycle `length` long: wrapped while it repeats, and at the end once it's complete
  pub(crate) fn local_time(self, time: f32, length: f32) -> f32 {
    if length <= 0.0 {
      return time;
    }
    match self {
      _ if self.is_complete(time, length) => length,
      Self::Once => time,
      Self::Times(_) | Self::Forever => time.rem_euclid(length),
    }
  }

  /// Whether every cycle `length` long has been played by `time`
  pub(crate) fn is_complete(self, time: f32, length: f32) -> bool {
    match self {
      Self::Once => time >= length,
      Self::Times(n) => time >= length * n as f32,
      Self::Forever => false,
    }
  }
}
