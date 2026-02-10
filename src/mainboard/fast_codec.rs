use tokio_util::bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use crate::protocol::{self, FastResponse};

pub struct FastCodec;

impl FastCodec {
  pub fn new() -> Self {
    FastCodec
  }
}

impl Decoder for FastCodec {
  type Item = FastResponse;
  type Error = std::io::Error;

  fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
    // Find the index of the \r byte
    if let Some(i) = src.iter().position(|&b| b == b'\r') {
      // Remove the data up to the \r from the buffer
      let data = src.split_to(i);
      // Remove the \r itself so it's not in the next message
      src.advance(1);

      // Parse to FastResponse
      let s = String::from_utf8_lossy(&data).to_string();
      if s.starts_with("WD:") {
        log::trace!("👾 -> 🖥️ : {}", s);
      } else {
        log::debug!("👾 -> 🖥️ : {}", s);
      }
      return Ok(protocol::parse(s));
    }
    // Not enough data for a full line yet
    Ok(None)
  }
}
