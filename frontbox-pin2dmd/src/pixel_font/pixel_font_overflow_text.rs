use frontbox::prelude::Rgba;
use frontbox_canvas::*;

use crate::PixelFontCharacterMap;

pub struct PixelFontOverflowText {
  pub text: String,
  pub color: Rgba<u8>,
  pub font: &'static PixelFontCharacterMap,
  pub spacing: u8,
}

impl Layer for PixelFontOverflowText {
  fn render<'a>(&self, canvas: &mut frontbox_canvas::CanvasView<'a>) {
    let ellipsis_width = self.font.glyph('…').map(|g| g.width).unwrap_or_else(|| 
      panic!(
        "Expected font {} to include ellipsis character '…' but did not.",
        self.font.name
      )
    ) as i32;
    let mut offset: i32 = 0;

    for c in self.text.chars() {
      // Check if there's not enough space to write this character AND the ellipsis. If not, render ellipsis and quit
      let glyph_width = self.font.glyph(c).map(|g| g.width).unwrap_or(0) as i32;
      let out_of_space = (offset + glyph_width + ellipsis_width) >= canvas.bounds.width as i32;

      let mut char_canvas = canvas.child_view(
        // shift origin left as characters are accumulated
        Position::new(offset, 0),
        canvas.bounds,
      );

      if out_of_space {
        let _ = self
          .font
          .render_char_image('…', self.color, &mut char_canvas) as i32
          + self.spacing as i32;
        break;
      }

      offset +=
        self.font.render_char_image(c, self.color, &mut char_canvas) as i32 + self.spacing as i32;
    }
  }
}
