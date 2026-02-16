pub enum FastAddress {
  Io(u8),
  Exp(u8, Option<u8>), // board, breakout
}
