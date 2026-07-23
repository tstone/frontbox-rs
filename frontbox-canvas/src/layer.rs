use image::DynamicImage;

use crate::*;

pub struct Layer {
  pub img: DynamicImage,
  pub horizontal: Horizontal,
  pub vertical: Vertical,
}

impl Layer {
  pub fn top_left(img: DynamicImage) -> Self {
    Self {
      img,
      horizontal: Horizontal::zero(),
      vertical: Vertical::zero(),
    }
  }

  pub fn offsets(&self, viewport: &Size<u32>) -> (i32, i32) {
    Self::viewport_offset(
      &Size::new(self.img.width(), self.img.height()),
      &self.horizontal,
      &self.vertical,
      viewport,
    )
  }

  pub fn viewport_offset(
    size: &Size<u32>,
    horizontal: &Horizontal,
    vertical: &Vertical,
    viewport: &Size<u32>,
  ) -> (i32, i32) {
    let viewport_offset_x = match horizontal {
      Horizontal::LeftOffset(e) => e.to_absolute(viewport.width as i32),
      Horizontal::RightOffset(e) => {
        (viewport.width as i32 - size.width as i32) - e.to_absolute(viewport.width as i32)
      }
      Horizontal::Centered => (viewport.width as i32 / 2) - (size.width as i32 / 2),
    };

    let viewport_offset_y = match vertical {
      Vertical::TopOffset(e) => e.to_absolute(viewport.height as i32),
      Vertical::BottomOffset(e) => {
        (viewport.height as i32 - size.height as i32) - e.to_absolute(viewport.height as i32)
      }
      Vertical::Centered => (viewport.height as i32 / 2) - (size.height as i32 / 2),
    };

    (viewport_offset_x, viewport_offset_y)
  }
}

pub trait LayerGenerator {
  fn generate(&self, viewport: &Size<u32>) -> Layer;
}
