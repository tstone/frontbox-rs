use palette::Srgb;

use crate::protocol::FastResponseError;
use crate::protocol::prelude::RawResponse;

/// Format expansion address with or without breakout nibble
pub fn expansion_addr(expansion_board: u8, breakout: Option<u8>) -> String {
  format!(
    "{:X}{}",
    expansion_board,
    if let Some(b) = breakout {
      format!("{:X}", b)
    } else {
      "".to_string()
    }
  )
}

pub fn srgb_to_hex(color: &Srgb) -> String {
  format!(
    "{:02X}{:02X}{:02X}",
    (color.red * 255.0) as u8,
    (color.green * 255.0) as u8,
    (color.blue * 255.0) as u8
  )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessedResponse {
  Processed,
  Failed,
}

impl ProcessedResponse {
  pub fn parse(raw: RawResponse) -> Result<ProcessedResponse, FastResponseError> {
    if raw.payload.to_lowercase() == "p" {
      Ok(ProcessedResponse::Processed)
    } else if raw.payload.to_lowercase() == "f" {
      Ok(ProcessedResponse::Failed)
    } else {
      Err(FastResponseError::InvalidFormat)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_srgb_to_hex() {
    let color = Srgb::new(1.0, 0.0, 0.0);
    let result = srgb_to_hex(&color);
    assert_eq!(result, "FF0000");
  }
}
