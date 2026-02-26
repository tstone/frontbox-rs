use crate::FastResponseError;
use crate::RawResponse;

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
