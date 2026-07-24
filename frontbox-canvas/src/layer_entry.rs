use image::{DynamicImage, ImageBuffer, Rgba};

use crate::*;

pub enum LayerEntry {
  Generated(Box<dyn LayerGenerator>),
  Static(Layer),
}

impl From<Layer> for LayerEntry {
  fn from(value: Layer) -> Self {
    LayerEntry::Static(value)
  }
}

impl From<DynamicImage> for LayerEntry {
  fn from(value: DynamicImage) -> Self {
    LayerEntry::Static(Layer::top_left(value))
  }
}

impl<T: LayerGenerator + 'static> From<T> for LayerEntry {
  fn from(generator: T) -> Self {
    LayerEntry::Generated(Box::new(generator))
  }
}

impl LayerEntry {
  pub fn render_at(
    entry: &LayerEntry,
    viewport: &Size<u32>,
    offset: Position,
    buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
  ) {
    let layer = match entry {
      LayerEntry::Static(l) => l,
      LayerEntry::Generated(g) => &g.generate(viewport),
    };
    let layer_img = layer.image.to_rgba8();
    let (viewport_offset_x, viewport_offset_y) = layer.absolute_offsets(viewport);

    for y in 0..layer_img.height() as i32 {
      for x in 0..layer_img.width() as i32 {
        let viewport_x = x + viewport_offset_x;
        let viewport_y = y + viewport_offset_y;

        // Skip pixels that are out of bounds of the viewport
        if viewport_x < 0
          || viewport_y < 0
          || viewport_x >= viewport.width as i32
          || viewport_y >= viewport.height as i32
        {
          continue;
        }

        let pixel = layer_img.get_pixel(x as u32, y as u32);

        // Skip transparent pixels
        if pixel[3] == 0 {
          continue;
        }

        // Translate into buffer-absolute coordinates only for the write
        let dest_x = viewport_x + offset.x;
        let dest_y = viewport_y + offset.y;

        buffer.put_pixel(dest_x as u32, dest_y as u32, *pixel);
      }
    }
  }

  pub fn render_all_at(
    layers: &Vec<LayerEntry>,
    viewport: &Size<u32>,
    offset: Position,
    buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
  ) {
    for entry in layers {
      // TODO: handle alpha blending of layers
      Self::render_at(entry, viewport, offset, buffer);
    }
  }
}
