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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SwitchState {
  Open,
  Closed,
}
