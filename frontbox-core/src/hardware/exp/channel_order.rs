use fast_protocol::Rgb;

/// The order an LED takes its red, green, and blue channels in, which depends on the chip and how it's wired.
///
/// The EXP board sends channels in the order it's given, so frontbox puts colors into this order before sending them.
///
/// Whether an LED has a white channel is set by its port, not here: on a port configured for an RGBW chip
/// ([`LedPort::sk6812`](crate::hardware::LedPort::sk6812)), the board shows the part shared by all three channels on
/// the white channel instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ChannelOrder {
  #[default]
  RGB,
  GRB,
  BRG,
}

impl ChannelOrder {
  /// Put `color` into the order this LED takes its channels in
  pub fn reorder(self, color: Rgb) -> Rgb {
    let Rgb { r, g, b } = color;
    match self {
      ChannelOrder::RGB => color,
      ChannelOrder::GRB => Rgb::new(g, r, b),
      ChannelOrder::BRG => Rgb::new(b, r, g),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_reorder() {
    let color = Rgb::new(1, 2, 3);
    assert_eq!(ChannelOrder::RGB.reorder(color), Rgb::new(1, 2, 3));
    assert_eq!(ChannelOrder::GRB.reorder(color), Rgb::new(2, 1, 3));
    assert_eq!(ChannelOrder::BRG.reorder(color), Rgb::new(3, 1, 2));
  }
}
