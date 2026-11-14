use ab_glyph::{Font as _, FontArc, GlyphId, OutlineCurve, Point};
use glam::Vec2;
use std::fmt;

use crate::scene::{Bounds, cubic};

/// Line segments per curve in a glyph's outline
const CURVE_STEPS: usize = 8;

/// The most pixels to an em a font is looked for on, past which it's too fine a grid to call a pixel font
const MAX_PIXELS_PER_EM: u32 = 64;

/// How far a font's points can be from a whole pixel, as a fraction of one, and still be on its grid. A grid that doesn't
/// divide the em evenly, such as 6 pixels to 1024 units, is rounded to whole font units.
const PIXEL_TOLERANCE: f32 = 0.02;

/// A TrueType or OpenType font to lay text out in. Clones share the font's data.
///
/// Text is drawn from the glyphs' outlines rather than from bitmaps, so it's a shape like any other: it scales and
/// rotates without blurring, takes strokes, and its edge is anti-aliased by the sample's footprint. There's no hinting,
/// so a font made for pixel displays, drawn at its design size ([`Self::pixel_size`]), is what looks crisp on a DMD.
#[derive(Clone)]
pub struct Font {
  face: FontArc,
  pixel_size: Option<f32>,
}

impl Font {
  pub fn new(bytes: Vec<u8>) -> Result<Self, InvalidFont> {
    let face = FontArc::try_from_vec(bytes).map_err(|_| InvalidFont)?;
    let pixel_size = pixel_size(&face);
    Ok(Self { face, pixel_size })
  }

  /// For a font drawn on a pixel grid, how many of its pixels there are to the em. At that size it's one scene unit to
  /// a pixel, and at whole multiples of it whole units to a pixel, so its edges land on whole units and it draws
  /// crisply. `None` for a font that isn't on a grid.
  pub fn pixel_size(&self) -> Option<f32> {
    self.pixel_size
  }

  /// How tall the capital letters are at `size`, in scene units, measured from "H". `None` for a font without one.
  pub fn cap_height(&self, size: f32) -> Option<f32> {
    let bounds = self.face.outline(self.face.glyph_id('H'))?.bounds;
    Some((bounds.max.y - bounds.min.y).abs() * self.scale(size))
  }

  /// How wide and tall `text` is on one line at `size`, in scene units. `size` is the font's em, as a font size is
  /// elsewhere, so a pixel font made on an 8-pixel em is one scene unit to a pixel at 8. A line is as tall as from the
  /// font's ascent to its descent, whatever the letters in it.
  pub fn measure(&self, text: &str, size: f32) -> Vec2 {
    Vec2::new(self.placed(text, size).1, self.ascent(size) + self.descent(size))
  }

  /// How far the font reaches above its baseline at `size`, in scene units
  pub(crate) fn ascent(&self, size: f32) -> f32 {
    self.face.ascent_unscaled() * self.scale(size)
  }

  /// How far the font reaches below its baseline at `size`, in scene units
  pub(crate) fn descent(&self, size: f32) -> f32 {
    -self.face.descent_unscaled() * self.scale(size)
  }

  /// How far apart the baselines of lines are at `size`, in scene units
  pub(crate) fn line_height(&self, size: f32) -> f32 {
    self.face.height_unscaled() * self.scale(size)
  }

  /// Lines of text at `size`, each from the start of its baseline, in local space with y down
  pub(crate) fn outline<'a>(&self, lines: impl IntoIterator<Item = (&'a str, Vec2)>, size: f32) -> TextOutline {
    let scale = self.scale(size);
    let placed = lines.into_iter().flat_map(|(text, start)| self.placed(text, size).0.into_iter().map(move |(id, pen)| (id, start + Vec2::X * pen)));
    let glyphs = placed
      .filter_map(|(id, pen)| {
        let to_local = |point: Point| pen + Vec2::new(point.x, -point.y) * scale;
        let edges: Vec<[Vec2; 2]> = self.face.outline(id)?.curves.iter().flat_map(|curve| flattened(curve, to_local)).collect();
        let bounds = edges.iter().flatten().fold(None, |bounds: Option<Bounds>, p| {
          Some(bounds.map_or(Bounds::new(*p, *p), |bounds| Bounds::new(bounds.min.min(*p), bounds.max.max(*p))))
        })?;
        Some(Glyph { bounds, edges })
      })
      .collect();
    TextOutline { glyphs }
  }

  /// Each glyph of `text` and where its pen position is along the line, with the line's width
  fn placed(&self, text: &str, size: f32) -> (Vec<(GlyphId, f32)>, f32) {
    let scale = self.scale(size);
    let mut pen = 0.0;
    let mut before: Option<GlyphId> = None;
    let placed = text
      .chars()
      .map(|c| {
        let id = self.face.glyph_id(c);
        pen += before.map_or(0.0, |before| self.face.kern_unscaled(before, id) * scale);
        let at = pen;
        pen += self.face.h_advance_unscaled(id) * scale;
        before = Some(id);
        (id, at)
      })
      .collect();
    (placed, pen)
  }

  /// Font units to scene units at `size`. 0 for a size of 0 or less.
  fn scale(&self, size: f32) -> f32 {
    let em = em(&self.face);
    if size > 0.0 && em > 0.0 { size / em } else { 0.0 }
  }
}

impl fmt::Debug for Font {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("Font")
  }
}

/// The bytes aren't a TrueType or OpenType font
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidFont;

impl fmt::Display for InvalidFont {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("not a TrueType or OpenType font")
  }
}

impl std::error::Error for InvalidFont {}

/// Laid-out text as the edges of its glyphs, in local space
#[derive(Debug, Clone)]
pub(crate) struct TextOutline {
  /// Glyphs with an outline. A space has none.
  glyphs: Vec<Glyph>,
}

impl TextOutline {
  /// Signed distance to the edge of the nearest glyph (negative inside any of them), with its gradient
  pub(crate) fn distance(&self, p: Vec2) -> (f32, Vec2) {
    // the glyph whose box is nearest first, so the others can mostly be skipped
    let first = (0..self.glyphs.len()).min_by(|a, b| box_distance(&self.glyphs[*a].bounds, p).total_cmp(&box_distance(&self.glyphs[*b].bounds, p)));
    let order = first.into_iter().chain((0..self.glyphs.len()).filter(|index| Some(*index) != first));
    let mut nearest = (f32::INFINITY, Vec2::X);
    for glyph in order.map(|index| &self.glyphs[index]) {
      // a glyph is inside its box, so it can't be any nearer than the box
      if box_distance(&glyph.bounds, p) >= nearest.0 {
        continue;
      }
      let distance = glyph.distance(p);
      if distance.0 < nearest.0 {
        nearest = distance;
      }
    }
    nearest
  }

  /// A box around every glyph. Can reach past the text's box, for a letter that hangs over its advance.
  pub(crate) fn extent(&self) -> Bounds {
    let mut boxes = self.glyphs.iter().map(|glyph| glyph.bounds);
    let first = boxes.next().unwrap_or(Bounds::new(Vec2::ZERO, Vec2::ZERO));
    boxes.fold(first, |all, bounds| Bounds::new(all.min.min(bounds.min), all.max.max(bounds.max)))
  }
}

/// One glyph's outline as line segments
#[derive(Debug, Clone)]
struct Glyph {
  bounds: Bounds,
  edges: Vec<[Vec2; 2]>,
}

impl Glyph {
  /// Signed distance to the glyph's edge, with its gradient. Inside is where the outline winds around the point (the
  /// non-zero rule), so a letter's counters, such as the hole in an "o", are outside.
  fn distance(&self, p: Vec2) -> (f32, Vec2) {
    let mut nearest = Vec2::splat(f32::INFINITY);
    let mut winding = 0;
    for &[a, b] in &self.edges {
      let edge = b - a;
      let length = edge.length_squared();
      let along = if length > 0.0 { ((p - a).dot(edge) / length).clamp(0.0, 1.0) } else { 0.0 };
      let offset = p - (a + edge * along);
      if offset.length_squared() < nearest.length_squared() {
        nearest = offset;
      }
      let side = edge.perp_dot(p - a);
      if a.y <= p.y && b.y > p.y && side > 0.0 {
        winding += 1;
      } else if a.y > p.y && b.y <= p.y && side < 0.0 {
        winding -= 1;
      }
    }
    let sign = if winding == 0 { 1.0 } else { -1.0 };
    (sign * nearest.length(), sign * nearest.normalize_or(Vec2::X))
  }
}

/// A font's em in font units
fn em(face: &FontArc) -> f32 {
  // a font without an em is very old, and its height is the nearest thing to one
  face.units_per_em().unwrap_or_else(|| face.height_unscaled())
}

/// The fewest pixels to an em that every point and advance of a font's printable characters lands on whole pixels of,
/// if there are any. Control characters, and the box for a missing one, are often left off the grid.
fn pixel_size(face: &FontArc) -> Option<f32> {
  let em = em(face);
  let mut ids: Vec<GlyphId> = face.codepoint_ids().filter(|(_, c)| !c.is_control()).map(|(id, _)| id).collect();
  ids.sort_unstable_by_key(|id| id.0);
  ids.dedup();
  let ids = ids.into_iter();
  let mut lengths: Vec<f32> = ids.clone().map(|id| face.h_advance_unscaled(id)).collect();
  for curve in ids.filter_map(|id| face.outline(id)).flat_map(|outline| outline.curves) {
    let points = match curve {
      OutlineCurve::Line(a, b) => vec![a, b],
      OutlineCurve::Quad(a, b, c) => vec![a, b, c],
      OutlineCurve::Cubic(a, b, c, d) => vec![a, b, c, d],
    };
    lengths.extend(points.into_iter().flat_map(|point| [point.x, point.y]));
  }
  if em <= 0.0 || lengths.iter().all(|length| *length == 0.0) {
    return None;
  }
  let on_grid = |pixels: f32| {
    let pixel = em / pixels;
    lengths.iter().all(|length| (length / pixel - (length / pixel).round()).abs() <= PIXEL_TOLERANCE)
  };
  (1..=MAX_PIXELS_PER_EM).map(|pixels| pixels as f32).find(|pixels| on_grid(*pixels))
}

/// A curve of a glyph's outline as line segments, each point moved into local space by `to_local`
fn flattened(curve: &OutlineCurve, to_local: impl Fn(Point) -> Vec2) -> Vec<[Vec2; 2]> {
  let points = match *curve {
    OutlineCurve::Line(a, b) => return vec![[to_local(a), to_local(b)]],
    // a quadratic is the cubic with its controls two thirds of the way to the quadratic's
    OutlineCurve::Quad(a, control, b) => {
      let (a, control, b) = (to_local(a), to_local(control), to_local(b));
      [a, a + (control - a) * 2.0 / 3.0, b + (control - b) * 2.0 / 3.0, b]
    }
    OutlineCurve::Cubic(a, b, c, d) => [to_local(a), to_local(b), to_local(c), to_local(d)],
  };
  let along: Vec<Vec2> = (0..=CURVE_STEPS).map(|step| cubic(points, step as f32 / CURVE_STEPS as f32)).collect();
  along.windows(2).map(|pair| [pair[0], pair[1]]).collect()
}

/// Signed distance to a box, negative inside it
fn box_distance(bounds: &Bounds, p: Vec2) -> f32 {
  let past = (p - bounds.center()).abs() - bounds.size() / 2.0;
  past.max(Vec2::ZERO).length() + past.max_element().min(0.0)
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  pub(crate) fn pixel_font() -> Font {
    Font::new(include_bytes!("../../tests/fonts/LCDBlock.ttf").to_vec()).unwrap()
  }

  #[test]
  fn bytes_that_arent_a_font_are_refused() {
    assert_eq!(Font::new(b"not a font".to_vec()).err(), Some(InvalidFont));
  }

  #[test]
  fn text_is_a_line_tall_and_as_wide_as_its_letters() {
    let font = pixel_font();
    let one = font.measure("A", 16.0);
    // ascent to descent, at 16 units to the em
    assert!((one.y - (682.0 + 192.0) * 16.0 / 1024.0).abs() < 1e-4);
    assert!(one.x > 0.0);
    assert!((font.measure("AAA", 16.0).x - 3.0 * one.x).abs() < 1e-3);
    assert!((font.measure("AAA", 32.0).x - 6.0 * one.x).abs() < 1e-3);
    assert_eq!(font.measure("", 16.0), Vec2::new(0.0, one.y));
  }

  #[test]
  fn a_line_starts_on_its_baseline() {
    let font = pixel_font();
    let start = Vec2::new(3.0, 20.0);
    let extent = font.outline([("HI", start)], 16.0).extent();
    // the letters sit on the baseline, within the ascent above it
    assert!((extent.max.y - start.y).abs() < 1e-4, "{extent:?}");
    assert!(extent.min.y >= start.y - font.ascent(16.0) - 1e-3, "{extent:?}");
    assert!(extent.min.x >= start.x && extent.max.x <= start.x + font.measure("HI", 16.0).x + 1e-3, "{extent:?}");
  }

  #[test]
  fn a_letter_is_inside_its_strokes_and_outside_its_counter() {
    // an "O" in a pixel font is a square ring, here with sides two and a half units wide
    let outline = pixel_font().outline([("O", Vec2::ZERO)], 40.0);
    let extent = outline.extent();
    let center = extent.center();
    assert!(outline.distance(center).0 > 0.0, "the counter is outside");
    let stroke = Vec2::new(extent.min.x + 1.0, center.y);
    assert!(outline.distance(stroke).0 < 0.0, "the left side is inside");
    assert!(outline.distance(Vec2::new(extent.min.x - 1.0, center.y)).0 > 0.0);
    assert!((outline.distance(Vec2::new(extent.min.x - 1.0, center.y)).0 - 1.0).abs() < 1e-3);
  }

  #[test]
  fn a_pixel_font_knows_how_many_pixels_its_em_is() {
    assert_eq!(pixel_font().pixel_size(), Some(16.0));
  }

  #[test]
  fn capitals_are_as_tall_as_an_h() {
    // LCDBlock's capitals are 7 of its pixels tall
    assert_eq!(pixel_font().cap_height(16.0), Some(7.0));
    assert_eq!(pixel_font().cap_height(32.0), Some(14.0));
  }

  #[test]
  fn a_space_has_no_outline() {
    let outline = pixel_font().outline([(" ", Vec2::ZERO)], 10.0);
    assert!(outline.glyphs.is_empty());
    assert_eq!(outline.distance(Vec2::ZERO).0, f32::INFINITY);
  }
}
