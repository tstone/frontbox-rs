use crate::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum EventResponse {
  Switch {
    switch_id: usize,
    state: SwitchState,
  },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum SwitchState {
  Open,
  Closed,
}

impl EventResponse {
  pub fn parse(raw: &RawResponse) -> Result<EventResponse, FastResponseError> {
    if raw.prefix == "-L" {
      switch_state::closed_response(&raw.payload)
    } else if raw.prefix == "/L" {
      switch_state::open_response(&raw.payload)
    } else {
      log::warn!("Unknown event type '{}'", raw.prefix);
      Err(FastResponseError::UnknownPrefix(raw.prefix.clone()))
    }
  }
}
