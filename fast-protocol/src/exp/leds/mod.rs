mod configure_led_port;
mod set_all_leds;
mod set_leds;
mod set_leds_binary;
mod set_multiple_leds;
mod set_multiple_leds_binary;
mod set_white;
mod set_white_binary;

pub use configure_led_port::*;
pub use set_all_leds::*;
pub use set_leds::*;
pub use set_leds_binary::*;
pub use set_multiple_leds::*;
pub use set_multiple_leds_binary::*;
pub use set_white::*;
pub use set_white_binary::*;

use crate::common::expansion_addr;

/// The most LEDs a binary LED command (RB, RD, RZ) can update, since the count is a single byte in `0x00`-`0x7F`
const MAX_BINARY_COUNT: usize = 0x7F;

/// Encodes a binary LED command of the form `<prefix>@<address>:<count>{<entry>...}`
fn binary_frames<T>(
  prefix: &str,
  expansion_board: u8,
  breakout: Option<u8>,
  entries: &[T],
  encode: impl Fn(&mut Vec<u8>, &T),
) -> Vec<u8> {
  let address = expansion_addr(expansion_board, breakout);
  let mut bytes = Vec::new();
  for chunk in entries.chunks(MAX_BINARY_COUNT) {
    bytes.extend(format!("{}@{}:", prefix, address).as_bytes());
    bytes.push(chunk.len() as u8);
    for entry in chunk {
      encode(&mut bytes, entry);
    }
  }
  bytes
}

/// Describes a binary LED command for logging, in the style of its ASCII sibling: `<prefix>@<address>:<part>,...`
fn describe_binary(
  prefix: &str,
  expansion_board: u8,
  breakout: Option<u8>,
  parts: impl Iterator<Item = String>,
) -> String {
  let address = expansion_addr(expansion_board, breakout);
  format!(
    "{}@{}:{}",
    prefix,
    address,
    parts.collect::<Vec<_>>().join(",")
  )
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum LedType {
  WS2812 = 0,
  SK6812 = 1,
  APA102 = 2,
}
