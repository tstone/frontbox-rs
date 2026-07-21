use std::collections::HashMap;

pub struct PixelFontCharacterMap {
  pub name: &'static str,
  pub height: u8,
  // key: code point/key code
  pub glyphs: HashMap<char, PixelFontGlyph>,
}

pub struct PixelFontGlyph {
  pub name: &'static str,
  pub width: u8,
  pub pixels: Vec<bool>,
}
