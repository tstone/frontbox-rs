use std::time::Duration;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum FastResponse {
  Unrecognized(String),
  Failed(String),
  Invalid(String),
  Processed(String),

  IdResponse {
    processor: String,
    product_number: String,
    firmware_version: String,
  },

  Switch {
    switch_id: usize,
    state: SwitchState,
  },
  SwitchReport {
    switches: Vec<SwitchState>,
  },

  WatchdogDisabled,
  WatchdogExpired,
  WatchdogRemaining(Duration),
}

impl FastResponse {
  /// Get the corresponding command prefix for this response, if applicable.
  pub fn command_prefix(&self) -> Option<String> {
    match self {
      FastResponse::Unrecognized(prefix) => Some(prefix.to_lowercase()),
      FastResponse::Failed(prefix) => Some(prefix.to_lowercase()),
      FastResponse::Invalid(prefix) => Some(prefix.to_lowercase()),
      FastResponse::Processed(prefix) => Some(prefix.to_lowercase()),
      FastResponse::IdResponse { .. } => Some("id".to_string()),
      FastResponse::Switch { .. } => None,
      FastResponse::SwitchReport { .. } => Some("sa".to_string()),
      FastResponse::WatchdogDisabled => Some("wd".to_string()),
      FastResponse::WatchdogExpired => Some("wd".to_string()),
      FastResponse::WatchdogRemaining(_) => Some("wd".to_string()),
    }
  }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SwitchState {
  Open,
  Closed,
}
