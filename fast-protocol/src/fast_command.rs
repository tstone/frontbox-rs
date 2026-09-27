//! ## Dispatch vs Command vs Query
//!
//!

use std::any::Any;
use std::fmt::Debug;

use crate::raw_response::RawResponse;
use crate::{FastResponseError, ProcessedResponse};

/// An instruction sent to the FAST hardware as fire and forget (e.g. set LED color)
pub trait FastStringDispatch: Debug + Send + Sync {
  fn to_string(&self) -> String;
}

/// An instruction sent to the FAST hardware as fire and forget (e.g. set LED color)
pub trait FastBinaryDispatch: Debug + Send + Sync {
  fn to_bytes(&self) -> Vec<u8>;
  fn as_any(&self) -> &dyn Any;
}

impl dyn FastBinaryDispatch {
  pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
    self.as_any().downcast_ref::<T>()
  }
}

impl<T: FastStringDispatch + 'static> FastBinaryDispatch for T {
  fn to_bytes(&self) -> Vec<u8> {
    self.to_string().as_bytes().to_vec()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

/// An instruction sent to the FAST hardware which awaits a success/fail acknowledgement (e.g. activate driver)
pub trait FastCommand: FastBinaryDispatch {
  fn prefix(&self) -> &'static str;

  fn parse(&self, raw: RawResponse) -> Result<ProcessedResponse, FastResponseError> {
    ProcessedResponse::parse(raw)
  }
}

/// An instruction sent to the FAST hardware which gets back a specific data response (e.g. "get switch state")
pub trait FastQuery: FastBinaryDispatch {
  type Response: Send + Sync;
  fn prefix(&self) -> &'static str;
  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError>;
}
