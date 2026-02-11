use crate::protocol::{prelude::*, switch_state};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum EventResponse {
  Switch {
    switch_id: usize,
    state: SwitchState,
  },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SwitchState {
  Open,
  Closed,
}

impl EventResponse {
  pub fn parse(raw: RawResponse) -> Result<EventResponse, FastResponseError> {
    if raw.prefix == "-L" {
      switch_state::closed_response(&raw.payload)
    } else if raw.prefix == "/L" {
      switch_state::open_response(&raw.payload)
    } else {
      log::warn!("Unknown event type '{}'", raw.prefix);
      Err(FastResponseError::UnknownPrefix(raw.prefix))
    }
  }
}
