use std::collections::HashMap;
use std::sync::LazyLock;

use crate::pixel_font::{PixelFontCharacterMap, PixelFontGlyph};

/// Sigi Regular 7px pixel font by Rasmus Andersson
/// CC0 Public Domain License
/// <https://github.com/rsms/sigi-pixel-font>
pub static SIGI_REGULAR_7PX_FONT: LazyLock<PixelFontCharacterMap> = LazyLock::new(|| {
  let mut glyphs = HashMap::new();

  glyphs.insert(
    '0',
    PixelFontGlyph {
      name: "0",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '1',
    PixelFontGlyph {
      name: "1",
      width: 2,
      pixels: vec![
        false, true, true, true, false, true, false, true, false, true, false, true, false, true,
      ],
    },
  );

  glyphs.insert(
    '2',
    PixelFontGlyph {
      name: "2",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, false, false, false,
        false, true, false, false, false, true, false, false, false, true, false, false, false,
        true, false, false, false, true, true, true, true, true,
      ],
    },
  );

  glyphs.insert(
    '3',
    PixelFontGlyph {
      name: "3",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, false, false, false,
        false, true, false, false, true, true, false, false, false, false, false, true, true,
        false, false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '4',
    PixelFontGlyph {
      name: "4",
      width: 5,
      pixels: vec![
        false, false, false, true, false, false, false, true, true, false, false, true, false,
        true, false, true, false, false, true, false, true, true, true, true, true, false, false,
        false, true, false, false, false, false, true, false,
      ],
    },
  );

  glyphs.insert(
    '5',
    PixelFontGlyph {
      name: "5",
      width: 5,
      pixels: vec![
        true, true, true, true, true, true, false, false, false, false, true, true, true, true,
        false, false, false, false, false, true, false, false, false, false, true, true, false,
        false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '6',
    PixelFontGlyph {
      name: "6",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        false, true, true, true, true, false, true, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '7',
    PixelFontGlyph {
      name: "7",
      width: 5,
      pixels: vec![
        true, true, true, true, true, false, false, false, false, true, false, false, false, false,
        true, false, false, false, true, false, false, false, false, true, false, false, false,
        true, false, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '8',
    PixelFontGlyph {
      name: "8",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, false, true, true, true, false, true, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '9',
    PixelFontGlyph {
      name: "9",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, false, true, true, true, true, false, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'A',
    PixelFontGlyph {
      name: "A",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, true, true, true, true, true, false, false, false, true, true, false, false,
        false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'B',
    PixelFontGlyph {
      name: "B",
      width: 5,
      pixels: vec![
        true, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, true, true, true, false, true, false, false, false, true, true, false, false,
        false, true, true, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'C',
    PixelFontGlyph {
      name: "C",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        false, true, false, false, false, false, true, false, false, false, false, true, false,
        false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'D',
    PixelFontGlyph {
      name: "D",
      width: 5,
      pixels: vec![
        true, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, true, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'E',
    PixelFontGlyph {
      name: "E",
      width: 5,
      pixels: vec![
        true, true, true, true, true, true, false, false, false, false, true, false, false, false,
        false, true, true, true, true, false, true, false, false, false, false, true, false, false,
        false, false, true, true, true, true, true,
      ],
    },
  );

  glyphs.insert(
    'F',
    PixelFontGlyph {
      name: "F",
      width: 5,
      pixels: vec![
        true, true, true, true, true, true, false, false, false, false, true, false, false, false,
        false, true, true, true, true, false, true, false, false, false, false, true, false, false,
        false, false, true, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    'G',
    PixelFontGlyph {
      name: "G",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        false, true, false, false, true, true, true, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'H',
    PixelFontGlyph {
      name: "H",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, true, true, true, true, true, true, false, false, false, true, true, false,
        false, false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'I',
    PixelFontGlyph {
      name: "I",
      width: 1,
      pixels: vec![true, true, true, true, true, true, true],
    },
  );

  glyphs.insert(
    'J',
    PixelFontGlyph {
      name: "J",
      width: 5,
      pixels: vec![
        false, false, false, false, true, false, false, false, false, true, false, false, false,
        false, true, false, false, false, false, true, false, false, false, false, true, true,
        false, false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'K',
    PixelFontGlyph {
      name: "K",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, true, false, true, false, true, false,
        false, true, true, false, false, false, true, false, true, false, false, true, false,
        false, true, false, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'L',
    PixelFontGlyph {
      name: "L",
      width: 4,
      pixels: vec![
        true, false, false, false, true, false, false, false, true, false, false, false, true,
        false, false, false, true, false, false, false, true, false, false, false, true, true,
        true, true,
      ],
    },
  );

  glyphs.insert(
    'M',
    PixelFontGlyph {
      name: "M",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, true, false, true, true, true, false, true, false,
        true, true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'N',
    PixelFontGlyph {
      name: "N",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, true, true, false, false,
        true, true, false, true, false, true, true, false, false, true, true, true, false, false,
        false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'O',
    PixelFontGlyph {
      name: "O",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'P',
    PixelFontGlyph {
      name: "P",
      width: 5,
      pixels: vec![
        true, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, true, true, true, false, true, false, false, false, false, true, false, false,
        false, false, true, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    'Q',
    PixelFontGlyph {
      name: "Q",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, false, false, false, true, true, false, false, false, true, true, false, false,
        true, true, false, true, true, true, true,
      ],
    },
  );

  glyphs.insert(
    'R',
    PixelFontGlyph {
      name: "R",
      width: 5,
      pixels: vec![
        true, true, true, true, false, true, false, false, false, true, true, false, false, false,
        true, true, true, true, true, false, true, false, true, false, false, true, false, false,
        true, false, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'S',
    PixelFontGlyph {
      name: "S",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, true, false, false, false,
        false, false, true, true, true, false, false, false, false, false, true, true, false,
        false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'T',
    PixelFontGlyph {
      name: "T",
      width: 5,
      pixels: vec![
        true, true, true, true, true, false, false, true, false, false, false, false, true, false,
        false, false, false, true, false, false, false, false, true, false, false, false, false,
        true, false, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    'U',
    PixelFontGlyph {
      name: "U",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, true, false, false, false, true, true, false, false, false, true, true, false,
        false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'V',
    PixelFontGlyph {
      name: "V",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, true, false, false,
        false, true, false, true, false, true, false, false, true, false, true, false, false,
        false, true, false, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    'W',
    PixelFontGlyph {
      name: "W",
      width: 7,
      pixels: vec![
        true, false, false, true, false, false, true, true, false, false, true, false, false, true,
        true, false, false, true, false, false, true, true, false, false, true, false, false, true,
        true, false, false, true, false, false, true, true, false, false, true, false, false, true,
        false, true, true, false, true, true, false,
      ],
    },
  );

  glyphs.insert(
    'X',
    PixelFontGlyph {
      name: "X",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, false, true, false, true,
        false, false, false, true, false, false, false, true, false, true, false, true, false,
        false, false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'Y',
    PixelFontGlyph {
      name: "Y",
      width: 5,
      pixels: vec![
        true, false, false, false, true, true, false, false, false, true, false, true, false, true,
        false, false, false, true, false, false, false, false, true, false, false, false, false,
        true, false, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    'Z',
    PixelFontGlyph {
      name: "Z",
      width: 5,
      pixels: vec![
        true, true, true, true, true, false, false, false, false, true, false, false, false, true,
        false, false, false, true, false, false, false, true, false, false, false, true, false,
        false, false, false, true, true, true, true, true,
      ],
    },
  );

  glyphs.insert(
    'Å',
    PixelFontGlyph {
      name: "Å",
      width: 5,
      pixels: vec![
        false, false, true, false, false, false, true, false, true, false, false, true, true, true,
        false, true, false, false, false, true, true, true, true, true, true, true, false, false,
        false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'Ä',
    PixelFontGlyph {
      name: "Ä",
      width: 5,
      pixels: vec![
        false, true, false, true, false, false, false, false, false, false, false, true, true,
        true, false, true, false, false, false, true, true, true, true, true, true, true, false,
        false, false, true, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    'Ö',
    PixelFontGlyph {
      name: "Ö",
      width: 5,
      pixels: vec![
        false, true, false, true, false, false, false, false, false, false, false, true, true,
        true, false, true, false, false, false, true, true, false, false, false, true, true, false,
        false, false, true, false, true, true, true, false,
      ],
    },
  );

  glyphs.insert(
    ' ',
    PixelFontGlyph {
      name: " ",
      width: 3,
      pixels: vec![
        false, false, false, false, false, false, false, false, false, false, false, false, false,
        false, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '!',
    PixelFontGlyph {
      name: "!",
      width: 1,
      pixels: vec![true, true, true, true, true, false, true],
    },
  );

  glyphs.insert(
    '@',
    PixelFontGlyph {
      name: "@",
      width: 7,
      pixels: vec![
        false, false, true, true, true, false, false, false, true, false, false, false, true,
        false, true, false, false, true, false, false, true, true, false, true, false, true, false,
        true, true, false, false, true, false, true, false, false, true, false, false, false,
        false, false, false, false, true, true, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '.',
    PixelFontGlyph {
      name: ".",
      width: 1,
      pixels: vec![false, false, false, false, false, false, true],
    },
  );

  glyphs.insert(
    '#',
    PixelFontGlyph {
      name: "#",
      width: 5,
      pixels: vec![
        false, true, false, true, false, false, true, false, true, false, true, true, true, true,
        true, false, true, false, true, false, true, true, true, true, true, false, true, false,
        true, false, false, true, false, true, false,
      ],
    },
  );

  glyphs.insert(
    '$',
    PixelFontGlyph {
      name: "$",
      width: 5,
      pixels: vec![
        false, false, true, false, false, false, true, true, true, true, true, false, true, false,
        false, false, true, true, true, false, false, false, true, false, true, true, true, true,
        true, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '%',
    PixelFontGlyph {
      name: "%",
      width: 7,
      pixels: vec![
        false, true, false, false, true, false, false, true, false, true, false, true, false,
        false, false, true, false, true, false, false, false, false, false, false, true, false,
        false, false, false, false, true, false, false, true, false, false, false, true, false,
        true, false, true, false, true, false, false, false, true, false,
      ],
    },
  );

  glyphs.insert(
    '^',
    PixelFontGlyph {
      name: "^",
      width: 5,
      pixels: vec![
        false, false, true, false, false, false, true, false, true, false, true, false, false,
        false, true, false, false, false, false, false, false, false, false, false, false, false,
        false, false, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '&',
    PixelFontGlyph {
      name: "&",
      width: 5,
      pixels: vec![
        false, false, true, true, false, false, true, false, false, true, false, true, false,
        false, true, false, true, true, false, false, true, false, true, false, true, true, false,
        false, true, false, false, true, true, true, true,
      ],
    },
  );

  glyphs.insert(
    '*',
    PixelFontGlyph {
      name: "*",
      width: 3,
      pixels: vec![
        true, false, true, false, true, false, true, false, true, false, false, false, false,
        false, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '(',
    PixelFontGlyph {
      name: "(",
      width: 2,
      pixels: vec![
        false, true, true, false, true, false, true, false, true, false, true, false, false, true,
      ],
    },
  );

  glyphs.insert(
    ')',
    PixelFontGlyph {
      name: ")",
      width: 2,
      pixels: vec![
        true, false, false, true, false, true, false, true, false, true, false, true, true, false,
      ],
    },
  );

  glyphs.insert(
    '-',
    PixelFontGlyph {
      name: "-",
      width: 3,
      pixels: vec![
        false, false, false, false, false, false, false, false, false, true, true, true, false,
        false, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '_',
    PixelFontGlyph {
      name: "_",
      width: 4,
      pixels: vec![
        false, false, false, false, false, false, false, false, false, false, false, false, false,
        false, false, false, false, false, false, false, false, false, false, false, true, true,
        true, true,
      ],
    },
  );

  glyphs.insert(
    '=',
    PixelFontGlyph {
      name: "=",
      width: 3,
      pixels: vec![
        false, false, false, false, false, false, true, true, true, false, false, false, true,
        true, true, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '+',
    PixelFontGlyph {
      name: "+",
      width: 5,
      pixels: vec![
        false, false, false, false, false, false, false, true, false, false, false, false, true,
        false, false, true, true, true, true, true, false, false, true, false, false, false, false,
        true, false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '[',
    PixelFontGlyph {
      name: "[",
      width: 2,
      pixels: vec![
        true, true, true, false, true, false, true, false, true, false, true, false, true, true,
      ],
    },
  );

  glyphs.insert(
    ']',
    PixelFontGlyph {
      name: "]",
      width: 2,
      pixels: vec![
        true, true, false, true, false, true, false, true, false, true, false, true, true, true,
      ],
    },
  );

  glyphs.insert(
    '{',
    PixelFontGlyph {
      name: "{",
      width: 3,
      pixels: vec![
        false, false, true, false, true, false, false, true, false, true, false, false, false,
        true, false, false, true, false, false, false, true,
      ],
    },
  );

  glyphs.insert(
    '}',
    PixelFontGlyph {
      name: "}",
      width: 3,
      pixels: vec![
        true, false, false, false, true, false, false, true, false, false, false, true, false,
        true, false, false, true, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '/',
    PixelFontGlyph {
      name: "/",
      width: 3,
      pixels: vec![
        false, false, true, false, false, true, false, false, true, false, true, false, false,
        true, false, true, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    '\\',
    PixelFontGlyph {
      name: "\\",
      width: 3,
      pixels: vec![
        true, false, false, true, false, false, true, false, false, false, true, false, false,
        true, false, false, false, true, false, false, true,
      ],
    },
  );

  glyphs.insert(
    '|',
    PixelFontGlyph {
      name: "|",
      width: 1,
      pixels: vec![true, true, true, true, true, true, true],
    },
  );

  glyphs.insert(
    '\'',
    PixelFontGlyph {
      name: "'",
      width: 1,
      pixels: vec![true, true, false, false, false, false, false],
    },
  );

  glyphs.insert(
    '"',
    PixelFontGlyph {
      name: "\"",
      width: 3,
      pixels: vec![
        true, false, true, true, false, true, true, false, true, false, false, false, false, false,
        false, false, false, false, false, false, false,
      ],
    },
  );

  glyphs.insert(
    '>',
    PixelFontGlyph {
      name: ">",
      width: 4,
      pixels: vec![
        true, false, false, false, false, true, false, false, false, false, true, false, false,
        false, false, true, false, false, true, false, false, true, false, false, true, false,
        false, false,
      ],
    },
  );

  glyphs.insert(
    '<',
    PixelFontGlyph {
      name: "<",
      width: 4,
      pixels: vec![
        false, false, false, true, false, false, true, false, false, true, false, false, true,
        false, false, false, false, true, false, false, false, false, true, false, false, false,
        false, true,
      ],
    },
  );

  glyphs.insert(
    ',',
    PixelFontGlyph {
      name: ",",
      width: 2,
      pixels: vec![
        false, false, false, false, false, false, false, false, false, false, false, true, true,
        false,
      ],
    },
  );

  glyphs.insert(
    '?',
    PixelFontGlyph {
      name: "?",
      width: 5,
      pixels: vec![
        false, true, true, true, false, true, false, false, false, true, false, false, false, true,
        false, false, false, true, false, false, false, false, true, false, false, false, false,
        false, false, false, false, false, true, false, false,
      ],
    },
  );

  glyphs.insert(
    ':',
    PixelFontGlyph {
      name: ":",
      width: 1,
      pixels: vec![false, false, true, false, false, true, false],
    },
  );

  glyphs.insert(
    ';',
    PixelFontGlyph {
      name: ";",
      width: 2,
      pixels: vec![
        false, false, false, false, false, true, false, false, false, false, false, true, true,
        false,
      ],
    },
  );

  glyphs.insert(
    '~',
    PixelFontGlyph {
      name: "~",
      width: 6,
      pixels: vec![
        false, false, false, false, false, false, false, true, true, false, false, true, true,
        false, false, true, true, false, false, false, false, false, false, false, false, false,
        false, false, false, false, false, false, false, false, false, false, false, false, false,
        false, false, false,
      ],
    },
  );

  glyphs.insert(
    '€',
    PixelFontGlyph {
      name: "€",
      width: 5,
      pixels: vec![
        false, false, true, true, true, false, true, false, false, false, true, true, true, true,
        false, false, true, false, false, false, true, true, true, true, false, false, true, false,
        false, false, false, false, true, true, true,
      ],
    },
  );

  glyphs.insert(
    '`',
    PixelFontGlyph {
      name: "`",
      width: 2,
      pixels: vec![
        true, false, false, true, false, false, false, false, false, false, false, false, false,
        false,
      ],
    },
  );

  PixelFontCharacterMap {
    name: "Sigi 7px Regular",
    height: 7,
    glyphs,
  }
});
