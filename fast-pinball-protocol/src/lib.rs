mod line_parser;
mod parser;
pub mod protocol;

pub use line_parser::LINE_PARSER;
pub use parser::{FastResponseError, SerialParser};
pub use protocol::FastResponse;
