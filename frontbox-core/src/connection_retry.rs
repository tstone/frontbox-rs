use std::time::Duration;

/// The wait after the first failed attempt to connect. It doubles with each failure after, up to `MAX_RETRY`.
const FIRST_RETRY: Duration = Duration::from_millis(250);
const MAX_RETRY: Duration = Duration::from_secs(5);

/// How long to wait after the `failures`th failed attempt to connect to a device before the next. Starts short, so a
/// device that's briefly busy comes back quickly, and backs off to a few seconds for one that's missing.
pub fn connection_retry_wait(failures: u32) -> Duration {
  FIRST_RETRY
    .saturating_mul(1 << failures.saturating_sub(1).min(8))
    .min(MAX_RETRY)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn retries_back_off_to_a_limit() {
    let waits: Vec<_> = (1..=7).map(connection_retry_wait).collect();
    assert_eq!(
      waits,
      [250, 500, 1000, 2000, 4000, 5000, 5000].map(Duration::from_millis)
    );
    assert_eq!(connection_retry_wait(1000), MAX_RETRY);
  }
}
