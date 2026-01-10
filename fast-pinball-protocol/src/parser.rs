use crate::protocol::FastResponse;

pub type SerialParser = fn(buffer: &mut Vec<u8>) -> Option<FastResponse>;

#[derive(Debug)]
pub enum FastResponseError {
  InvalidFormat,
  UnknownResponse,
}
