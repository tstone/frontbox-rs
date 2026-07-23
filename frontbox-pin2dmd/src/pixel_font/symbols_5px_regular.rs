// Auto-generated from font.json. Do not edit by hand.
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::pixel_font::{PixelFontCharacterMap, PixelFontGlyph};

pub static SYMBOLS_5PX_REGULAR: LazyLock<PixelFontCharacterMap> = LazyLock::new(|| {
  let mut glyphs = HashMap::new();

  glyphs.insert(
    '▲',
    PixelFontGlyph {
      name: "▲",
      width: 5,
      pixels: vec![
        false, false, true, false, false, false, true, true, true, false, false, true, true, true,
        false, true, true, true, true, true, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '▼',
    PixelFontGlyph {
      name: "▼",
      width: 5,
      pixels: vec![
        true, true, true, true, true, false, true, true, true, false, false, true, true, true,
        false, false, false, true, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '◀',
    PixelFontGlyph {
      name: "◀",
      width: 4,
      pixels: vec![
        false, false, false, true, false, true, true, true, true, true, true, true, false, true,
        true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    '▶',
    PixelFontGlyph {
      name: "▶",
      width: 4,
      pixels: vec![
        true, false, false, false, true, true, true, false, true, true, true, true, true, true,
        true, false, true, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '》',
    PixelFontGlyph {
      name: "》",
      width: 5,
      pixels: vec![
        true, false, true, false, false, false, true, false, true, false, false, false, true,
        false, true, false, true, false, true, false, true, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '《',
    PixelFontGlyph {
      name: "《",
      width: 5,
      pixels: vec![
        false, false, true, false, true, false, true, false, true, false, true, false, true, false,
        false, false, true, false, true, false, false, false, true, false, true,
      ],
    },
  );

  PixelFontCharacterMap {
    name: "Sigi 5px Symbols",
    height: 5,
    glyphs,
  }
});
