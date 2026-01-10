use crate::FastResponseError;
use crate::SerialParser;
use crate::protocol::FastResponse;
use crate::protocol::id;

fn parse_line(data: &[u8]) -> Result<FastResponse, FastResponseError> {
  if data.len() < 3 {
    return Err(FastResponseError::InvalidFormat);
  }

  let rem = &data[3..data.len() - 1];
  match &data[0..3] {
    b"ID:" => id::response(rem),
    _ => Err(FastResponseError::UnknownResponse),
  }
}

/// The "standard" parser for the Fast protocol. This parser reads lines terminated by '\r'
pub const LINE_PARSER: SerialParser = |buffer: &mut Vec<u8>| -> Option<FastResponse> {
  // seek for termination character
  if let Some(pos) = buffer.iter().position(|&b| b == b'\r') {
    // read up to and including termination character
    let line: Vec<u8> = buffer.drain(0..=pos).collect();
    log::debug!("👾 -> '{:?}'", line);

    match parse_line(&line) {
      Ok(response) => return Some(response),
      Err(e) => {
        log::error!("Failed to parse line: {:?}", e);
        return None;
      }
    }
  }

  log::trace!("Line parser: incomplete {:?}", buffer);
  None
};
