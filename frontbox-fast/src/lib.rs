pub mod protocol;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FastResponseError {
  InvalidFormat,
  UnknownResponse,
  UnknownPrefix(String),
}

pub use protocol::FastResponse;
pub use protocol::parse;
