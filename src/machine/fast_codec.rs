use tokio_util::bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use crate::protocol::raw_response::RawResponse;

pub struct FastRawCodec;

/// Decode incoming serial data into RawResponse structs. This is a low level parsing that just splits the raw string into
/// command, optional address, and payload.
impl FastRawCodec {
  pub fn new() -> Self {
    FastRawCodec
  }
}

impl Decoder for FastRawCodec {
  type Item = RawResponse;
  type Error = std::io::Error;

  fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
    if src.len() > 1024 {
      log::warn!(
        "Codec buffer has {} bytes without \\r terminator!",
        src.len()
      );
    }

    // Find the index of the \r byte
    if let Some(i) = src.iter().position(|&b| b == b'\r') {
      // Remove the data up to the \r from the buffer
      let data = src.split_to(i);
      // Remove the \r itself so it's not in the next message
      src.advance(1);

      // Parse to RawResponse
      let s = String::from_utf8_lossy(&data).to_string();

      // split on first :
      let mut parts = s.splitn(2, ':');
      let cmd = parts.next().unwrap_or("").to_string();
      let payload = parts.next().unwrap_or("").to_string();

      if cmd.contains("@") {
        let mut cmd_parts = cmd.splitn(2, '@');
        let cmd = cmd_parts.next().unwrap_or("").to_string();
        let address = cmd_parts.next().map(|s| s.to_string());
        return Ok(Some(RawResponse {
          prefix: cmd,
          address,
          payload,
        }));
      } else {
        return Ok(Some(RawResponse {
          prefix: cmd,
          address: None,
          payload,
        }));
      }
    }
    // Not enough data for a full line yet
    Ok(None)
  }
}
