use frontbox::prelude::Rgba;
use frontbox_canvas::*;

use crate::{PixelFontCharacterMap, PixelFontText, TextAlignment};

pub struct PixelFontMultiLineText {
  pub text: String,
  pub color: Rgba<u8>,
  pub font: &'static PixelFontCharacterMap,
  pub spacing: u8,
}

impl Layer for PixelFontMultiLineText {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    // break into lines
    let mut lines: Vec<String> = vec!["".to_string()];
    let mut current_index = 0;
    let mut current_line_length = 0;

    for word in self.text.split(' ') {
      if current_line_length + word.len() < canvas.bounds.width as usize {
        current_line_length += word.len();
        lines[current_index] += word;
      } else {
        current_line_length += word.len();
        lines.push(word.to_string());
        current_index += 1;
      }
    }

    let line_height = (self.font.height + self.spacing) as i32;
    for (i, line) in lines.iter().enumerate() {
      let line_offset = line_height * i as i32;
      let mut line_canvas = canvas.child_view(
        Position::new(0, line_offset),
        Size::new(
          canvas.bounds.width,
          canvas.bounds.height.saturating_sub(line_offset as u32),
        ),
      );

      let text = PixelFontText {
        text: line.clone(),
        color: self.color,
        font: self.font,
        spacing: self.spacing,
        alignment: TextAlignment::Left,
      };
      text.render(&mut line_canvas);
    }
  }
}
