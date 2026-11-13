use std::time::Instant;

use tokio::sync::oneshot;

use super::output::Output;
use super::output_device::Opened;

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
