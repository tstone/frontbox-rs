use frontbox::prelude::color_sequence::Extent;
use frontbox::prelude::*;
use image::{DynamicImage, RgbaImage};

use crate::*;

pub struct Container {
  pub size: Size<Extent<u32>>,
  pub horizontal: Horizontal,
  pub vertical: Vertical,
  pub padding: Padding,
  pub border: Option<Border>,
  pub layers: Vec<LayerEntry>,
}

impl Default for Container {
  fn default() -> Self {
    Self {
      size: Size::default(),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      padding: Padding::zero(),
      border: None,
      layers: Vec::new(),
    }
  }
}

impl Container {
  pub fn new(width: impl Into<Extent<u32>>, height: impl Into<Extent<u32>>) -> Self {
    Self::rect(Size::new(width.into(), height.into()))
  }

  pub fn rect(size: Size<Extent<u32>>) -> Self {
    Self {
      size,
      ..Default::default()
    }
  }

  pub fn full() -> Self {
    Self::new(Extent::full(), Extent::full())
  }

  pub fn with_horizontal(mut self, pos: Horizontal) -> Self {
    self.horizontal = pos;
    self
  }

  pub fn with_vertical(mut self, pos: Vertical) -> Self {
    self.vertical = pos;
    self
  }

  pub fn with_border(mut self, width: u8, color: Rgba<u8>) -> Self {
    self.border = Some(Border::new(width, color));
    self
  }

  pub fn with_padding(mut self, padding: Padding) -> Self {
    self.padding = padding;
    self
  }

  /// Add a layer above all other layers
  pub fn add(&mut self, layer: impl Into<LayerEntry>) {
    self.layers.push(layer.into());
  }
}

impl LayerGenerator for Container {
  fn generate(&self, viewport: &Size<u32>) -> Layer {
    // remove border and padding from viewport
    let border_width = match &self.border {
      Some(b) => b.width as u32,
      None => 0,
    };

    // resolved size of container based on viewport
    let container_width = self.size.width.to_absolute(viewport.width);
    let container_height = self.size.height.to_absolute(viewport.height);

    // actual renderable area minus padding and border
    let usable_area = Size {
      width: container_width - self.padding.left - self.padding.right - (border_width * 2),
      height: container_height - self.padding.top - self.padding.bottom - (border_width * 2),
    };

    log::trace!(
      "container size: {}x{}, viewport: {}x{}, usable: {}x{}",
      container_width,
      container_height,
      viewport.width,
      viewport.height,
      usable_area.width,
      usable_area.height,
    );

    let mut buffer = RgbaImage::new(container_width, container_height);
    LayerEntry::render_all_at(
      &self.layers,
      &usable_area,
      Position::from_u32(
        self.padding.left + border_width,
        self.padding.top + border_width,
      ),
      &mut buffer,
    );

    // render border
    if let Some(border) = &self.border {
      for w in 0..border.width as u32 {
        // top and bottom borders
        for x in w as u32..(container_width - w) {
          buffer.put_pixel(x, w as u32, border.color);
          buffer.put_pixel(x, (container_height - 1) - w, border.color);
        }
        // left and right borders
        for y in w as u32..(container_height - w) {
          buffer.put_pixel(w as u32, y, border.color);
          buffer.put_pixel((container_width - 1) - w, y, border.color);
        }
      }
    }

    Layer {
      image: DynamicImage::ImageRgba8(buffer),
      horizontal: self.horizontal,
      vertical: self.vertical,
    }
  }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
  left: u32,
  right: u32,
  top: u32,
  bottom: u32,
}

impl Padding {
  pub fn zero() -> Self {
    Padding::default()
  }

  pub fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
    Self {
      left,
      right,
      top,
      bottom,
    }
  }
}

pub struct Border {
  color: Rgba<u8>,
  width: u8,
}

impl Border {
  pub fn new(width: u8, color: Rgba<u8>) -> Self {
    Self { color, width }
  }
}
