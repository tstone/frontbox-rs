use std::time::Instant;

use frontbox_core::prelude::*;
use tokio::sync::oneshot;

use super::output::Output;
use super::output_device::Opened;

/// The wait after the first failed attempt to open the output. It doubles with each failure after, up to `MAX_RETRY`.
const FIRST_RETRY: Duration = Duration::from_millis(250);
const MAX_RETRY: Duration = Duration::from_secs(5);

/// Where the [`SoundManager`](super::SoundManager) is with the audio device
// there's only one, held by the manager, so its size doesn't matter
#[allow(clippy::large_enum_variant)]
pub(super) enum Connection {
  Connected(Output),
  /// An attempt to open the output is running. `failures` counts the attempts that failed before it.
  Connecting {
    result: oneshot::Receiver<Result<Opened, String>>,
    failures: u32,
  },
  /// The last attempt failed, and the next starts at `retry_at`
  Waiting { retry_at: Instant, failures: u32 },
}

/// How long to wait after the `failures`th failed attempt before the next
pub(super) fn retry_wait(failures: u32) -> Duration {
  FIRST_RETRY
    .saturating_mul(1 << failures.saturating_sub(1).min(8))
    .min(MAX_RETRY)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn retries_back_off_to_a_limit() {
    let waits: Vec<_> = (1..=7).map(retry_wait).collect();
    assert_eq!(
      waits,
      [250, 500, 1000, 2000, 4000, 5000, 5000].map(Duration::from_millis)
    );
    assert_eq!(retry_wait(1000), MAX_RETRY);
  }
}
