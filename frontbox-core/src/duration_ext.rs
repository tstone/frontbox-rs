use std::time::Duration;

pub trait DurationExt {
  /// How long a beat lasts at a tempo
  fn bpm(value: u16) -> Duration;
}

impl DurationExt for Duration {
  fn bpm(value: u16) -> Duration {
    Duration::from_secs_f32(60.0 / value as f32)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_beat_gets_shorter_as_the_tempo_rises() {
    assert_eq!(Duration::bpm(60), Duration::from_secs(1));
    assert_eq!(Duration::bpm(120), Duration::from_millis(500));
  }
}
