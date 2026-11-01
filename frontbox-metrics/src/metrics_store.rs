use std::error::Error;

use crate::MetricsBatch;

pub type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Where flushed metrics are written. Writes are blocking, so they run off the run loop on the store task.
pub trait MetricsStore: Send + 'static {
  /// Writes a batch as one transaction: all of it or none of it
  fn write(&mut self, batch: &MetricsBatch) -> StoreResult<()>;
}
