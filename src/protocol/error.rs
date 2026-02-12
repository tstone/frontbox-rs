use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FastResponseError {
  InvalidFormat,
  UnknownResponse,
  UnknownPrefix(String),
  Timeout,
}

impl Display for FastResponseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FastResponseError::InvalidFormat => write!(f, "Invalid response format"),
      FastResponseError::UnknownResponse => write!(f, "Unknown response"),
      FastResponseError::UnknownPrefix(prefix) => {
        write!(f, "Unknown response prefix: {}", prefix)
      }
      FastResponseError::Timeout => write!(f, "Response timed out"),
    }
  }
}
