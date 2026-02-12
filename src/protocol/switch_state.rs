use crate::protocol::{EventResponse, FastResponseError, SwitchState};

pub fn open_response(data: &str) -> Result<EventResponse, FastResponseError> {
  // convert hex string into decimal ID
  match usize::from_str_radix(data, 16) {
    Ok(id) => Ok(EventResponse::Switch {
      switch_id: id,
      state: SwitchState::Open,
    }),
    Err(_) => Err(FastResponseError::InvalidFormat),
  }
}

pub fn closed_response(data: &str) -> Result<EventResponse, FastResponseError> {
  // convert hex string into decimal ID
  match usize::from_str_radix(data, 16) {
    Ok(id) => Ok(EventResponse::Switch {
      switch_id: id,
      state: SwitchState::Closed,
    }),
    Err(_) => Err(FastResponseError::InvalidFormat),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_open_response() {
    let result = open_response("1A");
    assert_eq!(
      result,
      Ok(EventResponse::Switch {
        switch_id: 26,
        state: SwitchState::Open
      })
    );
  }

  #[test]
  fn test_closed_response() {
    let result = closed_response("FF");
    assert_eq!(
      result,
      Ok(EventResponse::Switch {
        switch_id: 255,
        state: SwitchState::Closed
      })
    );
  }
}
