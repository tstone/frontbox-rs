use crate::protocol::FastResponseError;
use crate::protocol::raw_response::RawResponse;

pub trait FastCommand {
  type Response;
  fn prefix() -> &'static str;
  fn to_string(&self) -> String;
  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessedResponse {
  Processed,
  Failed,
}
