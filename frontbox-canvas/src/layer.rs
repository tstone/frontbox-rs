//! # Layers
//!
//! The layers module features two main constructs that are key to understanding canvas rendering:
//!
//! 1. `Layer` - A static image with 2d positioning
//! 2. `LayerGenerator` - Something which can create a layer on demand, for a given viewport size
//!
//! The difference between these can often be resolved with a single question: Do I need to know the size
//! of the viewport in order to render this? Static assets like images or glyphs of a pixel font typically
//! do not. Dynamic assets, like a frame which fills the viewport and has a gradient fill, do.
//!
//! Some resources may have methods that use both. A pixel font might return a Layer for individual glyphs
//! but may also have a LayerGenerator to be able to support an "overflow text" functionality where longer
//! text truncates with an ellipsis. The latter needs to know the size of the viewport (parent) in order to
//! effectively render itself.
//!
//! - If it can be statically generated without knowledge of the parent -> `Layer`
//! - If it needs to know something about the parent size -> `LayerGenerator`

use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::Rgba;
use image::DynamicImage;

use crate::*;

/// A layer represents a static image with 2d positioning data
pub struct Layer {
  pub image: DynamicImage,
  pub horizontal: Horizontal,
  pub vertical: Vertical,
}

impl Layer {
  pub fn top_left(image: DynamicImage) -> Self {
    Self {
      image,
      horizontal: Horizontal::zero(),
      vertical: Vertical::zero(),
    }
  }

  pub fn size(&self) -> Size<u32> {
    Size::new(self.image.width(), self.image.height())
  }

  pub fn image_mut(&mut self) -> &mut DynamicImage {
    &mut self.image
  }

  pub fn fliph(mut self) -> Self {
    self.image = DynamicImage::fliph(&self.image);
    self
  }

  pub fn flipv(mut self) -> Self {
    self.image = DynamicImage::flipv(&self.image);
    self
  }

  pub fn recolor(mut self, color: Rgba<u8>) -> Self {
    let mut buffer = self.image.to_rgba8();

    for pixel in buffer.pixels_mut() {
      let brightness =
        (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0;
      pixel[0] = (color[0] as f32 * brightness) as u8;
      pixel[1] = (color[1] as f32 * brightness) as u8;
      pixel[2] = (color[2] as f32 * brightness) as u8;
    }

    self.image = DynamicImage::ImageRgba8(buffer);
    self
  }

  pub fn recolor_gradient(mut self, stops: Vec<GradientStop>, angle: f32) -> Self {
    let gradient = Gradient2d::new(stops, angle, self.size());
    let img_width = self.size().width;
    let mut buffer = self.image.to_rgba8();

    for (index, pixel) in buffer.pixels_mut().enumerate() {
      let brightness =
        (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0;
      let gradient_pixel = gradient.sample_at_index(index, img_width);

      pixel[0] = (gradient_pixel[0] as f32 * brightness) as u8;
      pixel[1] = (gradient_pixel[1] as f32 * brightness) as u8;
      pixel[2] = (gradient_pixel[2] as f32 * brightness) as u8;
    }

    self.image = DynamicImage::ImageRgba8(buffer);
    self
  }

  pub fn with_horizontal(mut self, horizontal: Horizontal) -> Self {
    self.horizontal = horizontal;
    self
  }

  pub fn horizontal_mut(&mut self) -> &mut Horizontal {
    &mut self.horizontal
  }

  pub fn with_vertical(mut self, vertical: Vertical) -> Self {
    self.vertical = vertical;
    self
  }

  pub fn vertical_mut(&mut self) -> &mut Vertical {
    &mut self.vertical
  }

  pub fn absolute_offsets(&self, viewport: &Size<u32>) -> (i32, i32) {
    Self::viewport_offsets(
      &Size::new(self.image.width(), self.image.height()),
      &self.horizontal,
      &self.vertical,
      viewport,
    )
  }

  pub fn viewport_offsets(
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

/// A layer generator is something which can create a layer on demand, for a given viewport size.
pub trait LayerGenerator {
  fn generate(&self, viewport: &Size<u32>) -> Layer;
}
