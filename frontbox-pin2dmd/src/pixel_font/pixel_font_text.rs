use frontbox::prelude::Rgba;
use frontbox_canvas::*;

use crate::PixelFontCharacterMap;

pub struct PixelFontText {
  pub text: String,
  pub color: Rgba<u8>,
  pub font: &'static PixelFontCharacterMap,
  pub spacing: u8,
  pub alignment: TextAlignment,
}

impl PixelFontText {
  fn render_left_alignment(&self, canvas: &mut CanvasView, mut offset: i32) {
    for c in self.text.chars() {
      let mut char_canvas = canvas.child_view(
        // shift origin left as characters are accumulated
        Position::new(offset, 0),
        canvas.bounds,
      );
      offset +=
        self.font.render_char_image(c, self.color, &mut char_canvas) as i32 + self.spacing as i32;
    }
  }

  fn render_center_alignment(&self, canvas: &mut CanvasView) {
    let left_offset = (canvas.bounds.width as u16 / 2) - (self.total_width() / 2);
    self.render_left_alignment(canvas, left_offset as i32);
  }

  fn render_right_alignment(&self, canvas: &mut CanvasView) {
    // starting from the max width, render the right-most characters, offsetting to the left by the width of the char
    let mut offset = canvas.bounds.width as i32;
    for c in self.text.chars().rev() {
      offset = offset.saturating_sub(self.font.glyph(c).map(|gl| gl.width).unwrap_or(0) as i32);
      let mut char_canvas = canvas.child_view(
        // shift origin left as characters are accumulated
        Position::new(offset, 0),
        canvas.bounds,
      );
      let _ = self.font.render_char_image(c, self.color, &mut char_canvas);
      offset = offset.saturating_sub(self.spacing as i32);
    }
  }

  pub fn total_width(&self) -> u16 {
    self
      .text
      .chars()
      .fold(0, |acc, c| match self.font.glyph(c) {
        Some(glyph) => acc + glyph.width as u16,
        None => acc,
      })
  }
}

impl Layer for PixelFontText {
  fn render<'a>(&self, canvas: &mut CanvasView) {
    match self.alignment {
      TextAlignment::Left => self.render_left_alignment(canvas, 0),
      TextAlignment::Centered => self.render_center_alignment(canvas),
      TextAlignment::Right => self.render_right_alignment(canvas),
    }
  }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextAlignment {
  #[default]
  Left,
  Centered,
  Right,
}
