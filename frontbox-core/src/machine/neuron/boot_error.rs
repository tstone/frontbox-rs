use std::fmt::Display;

use fast_protocol::{FastResponseError, ProcessedResponse};

use crate::machine::serial_interface::SerialError;

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
  /// The serial port went away during the step. Retrying on the same port won't help.
  PortLost { step: String },
}

impl BootError {
  /// The step got no usable response, either because the hardware didn't give one or the port is gone
  pub(crate) fn no_response(step: String, error: SerialError) -> BootError {
    match error {
      SerialError::Lost => BootError::PortLost { step },
      SerialError::Response(error) => BootError::NoResponse { step, error },
    }
  }

  /// Turn the result of a command into a `BootError` unless the hardware processed it
  pub(crate) fn processed(
    step: impl FnOnce() -> String,
    result: Result<ProcessedResponse, SerialError>,
  ) -> Result<(), BootError> {
    match result {
      Ok(ProcessedResponse::Processed) => Ok(()),
      Ok(ProcessedResponse::Failed) => Err(BootError::Failed { step: step() }),
      Err(error) => Err(BootError::no_response(step(), error)),
    }
  }

  /// Whether trying the step again could succeed
  pub fn is_retryable(&self) -> bool {
    matches!(
      self,
      BootError::NoResponse { .. } | BootError::Failed { .. }
    )
  }
}

impl Display for BootError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BootError::NoResponse { step, error } => write!(f, "{step}: {error}"),
      BootError::Failed { step } => write!(f, "{step}: the hardware reported a failure"),
      BootError::Misconfigured(reason) => write!(f, "{reason}"),
      BootError::PortLost { step } => write!(f, "{step}: the port was lost"),
    }
  }
}

impl std::error::Error for BootError {}
