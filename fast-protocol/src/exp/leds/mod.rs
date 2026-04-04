mod configure_led_port;
mod set_all_leds;
mod set_leds;
mod set_multiple_leds;
mod set_white;

pub use configure_led_port::*;
pub use set_all_leds::*;
pub use set_leds::*;
pub use set_multiple_leds::*;
pub use set_white::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LedType {
  WS2812 = 0,
  SK6812 = 1,
  APA102 = 2,
}
