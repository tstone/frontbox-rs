pub mod protocol;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FastResponseError {
  InvalidFormat,
  UnknownResponse,
}

pub use protocol::FastResponse;
