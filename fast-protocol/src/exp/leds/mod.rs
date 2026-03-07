mod configure_led_port;
mod set_led_colors;

pub use configure_led_port::*;
pub use set_led_colors::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedType {
  WS2812 = 0,
  SK6812 = 1,
  APA102 = 2,
}
