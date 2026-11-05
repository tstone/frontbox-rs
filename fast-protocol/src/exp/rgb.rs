/// The red, green, and blue an LED is set to, in the order they're sent on the wire.
///
/// An LED on a port configured for an RGBW chip (`LedType::SK6812`) gets its white channel from these too: the EXP
/// board takes the part shared by all three channels and shows it on the white channel instead (`FFFFFF` is shown
/// as white only, `FFFFFE` as `010100` plus `FE` white). Set the white channel explicitly, on top of that, with `RZ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Rgb {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

impl Rgb {
  pub const fn new(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b }
  }

  pub const fn black() -> Self {
    Self::new(0, 0, 0)
  }

  pub fn to_hex(&self) -> String {
    format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
  }
}
