use std::fmt::Display;

use fast_protocol::{FastResponseError, ProcessedResponse};

#[derive(Debug, Clone, PartialEq)]
pub enum BootError {
  NoResponse {
    step: String,
    error: FastResponseError,
  },
  /// The hardware responded, but reported that it couldn't do what was asked
  Failed { step: String },
  /// The hardware doesn't match the configuration. Retrying won't help.
  Misconfigured(String),
}

impl BootError {
  /// Turn the result of a command into a `BootError` unless the hardware processed it
  pub(crate) fn processed(
    step: impl FnOnce() -> String,
    result: Result<ProcessedResponse, FastResponseError>,
  ) -> Result<(), BootError> {
    match result {
      Ok(ProcessedResponse::Processed) => Ok(()),
      Ok(ProcessedResponse::Failed) => Err(BootError::Failed { step: step() }),
      Err(error) => Err(BootError::NoResponse {
        step: step(),
        error,
      }),
    }
  }
}

impl Display for BootError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BootError::NoResponse { step, error } => write!(f, "{step}: {error}"),
      BootError::Failed { step } => write!(f, "{step}: the hardware reported a failure"),
      BootError::Misconfigured(reason) => write!(f, "{reason}"),
    }
  }
}

impl std::error::Error for BootError {}
