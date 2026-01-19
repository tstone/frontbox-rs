#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FastResponseError {
  InvalidFormat,
  UnknownResponse,
  UnknownPrefix(String),
}
