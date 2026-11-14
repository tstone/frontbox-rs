use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Font, TextOutline};

/// Where each line of a [`ShapeNode::Text`] sits across its box
///
/// [`ShapeNode::Text`]: crate::scene::ShapeNode::Text
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
  Left,
  #[default]
  Center,
  Right,
}

impl TextAlign {
  /// How much of the room left over goes before the line
  fn fraction(self) -> f32 {
    match self {
      Self::Left => 0.0,
      Self::Center => 0.5,
      Self::Right => 1.0,
    }
  }
}

/// Where the lines of a [`ShapeNode::Text`] sit, together, up and down its box
///
/// [`ShapeNode::Text`]: crate::scene::ShapeNode::Text
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerticalAlign {
  Top,
  #[default]
  Middle,
  Bottom,
}

impl VerticalAlign {
  fn fraction(self) -> f32 {
    match self {
      Self::Top => 0.0,
      Self::Middle => 0.5,
      Self::Bottom => 1.0,
    }
  }
}

/// `text` laid out in a box `frame` wide and tall around the origin, at `size`, with y down. Lines are a line height
/// apart, and are aligned by their capitals: from the top of the first line's to the last line's baseline, or the
/// font's ascent for a font without an "H". A font's ascent can reach well above its capitals, which would push the
/// letters down from where they're aligned. Letters that go below the baseline hang past it. `spacing` is room added
/// between characters (x) and between lines (y), and can be negative.
///
/// In a pixel font, spacing is rounded to whole font pixels, and each line starts on a grid so it stays crisp wherever
/// the box and alignment put it. `moved_to` is where the box's center is in scene space when it's only moved, not
/// scaled or turned. Then the grid is whole scene units, as a display's pixels are, or the font's pixels where those are
/// smaller. Otherwise it's the font's pixels, counted from the box's top left.
pub(crate) fn layout_text(
  font: &Font,
  text: &str,
  size: f32,
  frame: Vec2,
  align: TextAlign,
  vertical: VerticalAlign,
  spacing: Vec2,
  moved_to: Option<Vec2>,
) -> TextOutline {
  let pixel = font.pixel_size().filter(|_| size > 0.0).map(|pixels| size / pixels);
  let whole = |length: f32| pixel.map_or(length, |pixel| (length / pixel).round() * pixel);
  let kerning = whole(spacing.x);
  let lines = wrap_text(font, text, size, kerning, frame.x);
  let corner = -frame / 2.0;
  // the grid's size, and where one of its points is in local space
  let grid = pixel.map(|pixel| match moved_to {
    Some(center) => (pixel.min(1.0), -center),
    None => (pixel, corner),
  });
  let snap = |point: Vec2| grid.map_or(point, |(step, from)| from + ((point - from) / step).round() * step);
  let cap = font.cap_height(size).unwrap_or_else(|| font.ascent(size));
  // whole font pixels apart, so every line is on the grid the first is
  let advance = whole(font.line_height(size) + spacing.y);
  let height = cap + advance * (lines.len() - 1) as f32;
  let top = vertical.fraction() * (frame.y - height);
  let placed = lines.iter().enumerate().map(|(i, line)| {
    let x = align.fraction() * (frame.x - font.measure(line, size, kerning).x);
    let y = top + cap + advance * i as f32;
    (line.as_str(), snap(corner + Vec2::new(x, y)))
  });
  font.outline(placed, size, kerning)
}

/// `text` as lines: one for each line break, each broken between words where the next word would run past `width`, with
/// `kerning` between characters. A word wider than `width` is a line to itself. Always at least one line.
pub fn wrap_text(font: &Font, text: &str, size: f32, kerning: f32, width: f32) -> Vec<String> {
  let mut lines = Vec::new();
  for paragraph in text.split('\n').map(|paragraph| paragraph.trim_end_matches('\r')) {
    let mut line: Option<String> = None;
    for word in paragraph.split(' ') {
      line = Some(match line {
        None => word.into(),
        Some(line) => {
          let longer = format!("{line} {word}");
          if font.measure(&longer, size, kerning).x <= width {
            longer
          } else {
            lines.push(line);
            word.into()
          }
        }
      });
    }
    lines.extend(line);
  }
  lines
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::font::tests::pixel_font;

  #[test]
  fn lines_break_at_line_breaks_and_between_words_that_run_past_the_width() {
    let font = pixel_font();
    let letter = font.measure("A", 16.0, 0.0).x;
    assert_eq!(wrap_text(&font, "AB CD\nEF", 16.0, 0.0, 100.0 * letter), ["AB CD", "EF"]);
    assert_eq!(wrap_text(&font, "AB CD EF", 16.0, 0.0, 5.0 * letter), ["AB CD", "EF"]);
    assert_eq!(wrap_text(&font, "ABCDEFG H", 16.0, 0.0, 3.0 * letter), ["ABCDEFG", "H"]);
    assert_eq!(wrap_text(&font, "", 16.0, 0.0, 0.0), [""]);
    assert_eq!(wrap_text(&font, "A\r\n\nB", 16.0, 0.0, 10.0), ["A", "", "B"]);
  }

  #[test]
  fn kerning_counts_toward_where_lines_break() {
    let font = pixel_font();
    let letter = font.measure("A", 16.0, 0.0).x;
    assert_eq!(wrap_text(&font, "AB CD", 16.0, 0.0, 5.0 * letter), ["AB CD"]);
    assert_eq!(wrap_text(&font, "AB CD", 16.0, 1.0, 5.0 * letter), ["AB", "CD"]);
  }
}
