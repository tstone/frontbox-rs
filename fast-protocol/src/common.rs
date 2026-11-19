use crate::FastResponseError;
use crate::RawResponse;

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
