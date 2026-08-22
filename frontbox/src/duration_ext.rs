use std::time::Duration;


pub trait DurationExt {
  fn bpm(value: u16) -> Duration;
}

impl DurationExt for Duration {
  fn bpm(value: u16) -> Duration {
    Duration::from_millis(((value as f32 / 60.0) * 1000.0) as u64)
  }
}