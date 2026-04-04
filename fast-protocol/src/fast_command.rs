use std::any::Any;
use std::fmt::Debug;

use crate::FastResponseError;
use crate::raw_response::RawResponse;

pub trait FastStringCommand: Debug + Send + Sync {
  fn to_string(&self) -> String;
}

pub trait FastBinaryCommand: Debug + Send + Sync {
  fn to_bytes(&self) -> Vec<u8>;
}

impl<T: FastStringCommand> FastBinaryCommand for T {
  fn to_bytes(&self) -> Vec<u8> {
    self.to_string().as_bytes().to_vec()
  }
}

pub trait FastRequestCommand: FastBinaryCommand {
  type Response: Send + Sync;
  fn prefix() -> &'static str;
  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError>;
}

pub trait FastAnyRequestCommand: FastBinaryCommand {
  fn cmd_prefix(&self) -> &'static str;
  fn parse_any(&self, raw: RawResponse) -> Result<Box<dyn Any + Send + Sync>, FastResponseError>;
}

impl<T: FastRequestCommand> FastAnyRequestCommand for T
where
  T: Send + Sync,
  T::Response: Send + Sync + 'static,
{
  fn cmd_prefix(&self) -> &'static str {
    Self::prefix()
  }

  fn parse_any(&self, raw: RawResponse) -> Result<Box<dyn Any + Send + Sync>, FastResponseError> {
    self
      .parse(raw)
      .map(|resp| Box::new(resp) as Box<dyn Any + Send + Sync>)
  }
}
