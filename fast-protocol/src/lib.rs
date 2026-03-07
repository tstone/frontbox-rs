mod common;
mod error;
mod event_response;
pub mod exp;
mod fast_command;
pub mod net;
mod raw_response;

pub use crate::common::ProcessedResponse;
pub use crate::exp::prelude::*;
pub use crate::fast_command::*;
pub use crate::net::prelude::*;
pub use crate::raw_response::RawResponse;
pub use error::FastResponseError;
pub use event_response::*;

pub enum FastAddress {
  Io(u8),
  Exp(u8, Option<u8>), // board, breakout
}
