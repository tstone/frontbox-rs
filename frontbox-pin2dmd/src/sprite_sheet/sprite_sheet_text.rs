use frontbox_canvas::*;

use crate::SpriteSheetFont;

pub struct SpriteSheetFontText {
  pub text: String,
  pub font: &'static SpriteSheetFont,
  pub spacing: u8,
}

impl Layer for SpriteSheetFontText {
  fn render<'a>(&self, canvas: &mut frontbox_canvas::CanvasView<'a>) {
    let mut offset = 0;
    for c in self.text.chars() {
      let mut char_canvas = canvas.child_view(
        // shift origin left as characters are accumulated
        Position::new(canvas.origin.x + offset, canvas.origin.y),
        canvas.bounds,
      );
      offset += self.font.render_char_image(c, &mut char_canvas) as i32 + self.spacing as i32;
    }
  }
}
