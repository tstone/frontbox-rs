use frontbox::prelude::Rgba;
use frontbox_canvas::*;

use crate::PixelFontCharacterMap;

pub struct PixelFontText {
  pub text: String,
  pub color: Rgba<u8>,
  pub font: &'static PixelFontCharacterMap,
  pub spacing: u8,
}

impl Layer for PixelFontText {
  fn render<'a>(&self, canvas: &mut CanvasView) {
    let mut offset = 0;
    for c in self.text.chars() {
      let mut char_canvas = canvas.child_view(
        // shift origin left as characters are accumulated
        Position::new(canvas.origin.x + offset, canvas.origin.y),
        canvas.bounds,
      );
      offset +=
        self.font.render_char_image(c, self.color, &mut char_canvas) as i32 + self.spacing as i32;
    }
  }
}
