use fast_protocol::Color;

use crate::{Bitmap, HardwareTag, Illumination, Point, RenderableGeom};

#[derive(Debug, Clone)]
pub struct Led {
  name: &'static str,
  tags: Vec<Box<dyn HardwareTag>>,
  geom: Option<RenderableGeom>,
}

impl Led {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      geom: None,
    }
  }

  pub fn tagged(mut self, tag: impl HardwareTag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn geom(mut self, x: f32, y: f32, diam: f32) -> Self {
    self.geom = Some(RenderableGeom::Circle {
      center: Point(x, y),
      radius: diam / 2.0,
    });
    self
  }
}

impl Illumination for Led {
  fn name(&self) -> &'static str {
    self.name
  }

  fn tags(&self) -> &Vec<Box<dyn HardwareTag>> {
    &self.tags
  }

  fn geom(&self) -> Option<&RenderableGeom> {
    self.geom.as_ref()
  }

  fn led_count(&self) -> u8 {
    1
  }

  fn render_bitmap(&self, bitmap: &Bitmap) -> Vec<fast_protocol::Color> {
    // Use the center pixel's color for the whole LED since it's just a single point
    let center_x = bitmap.width as f32 / 2.0;
    let center_y = bitmap.height as f32 / 2.0;
    let center_index =
      (center_y.floor() as usize * bitmap.width as usize + center_x.floor() as usize) * 3;
    vec![Color::rgb(
      bitmap.data[center_index],
      bitmap.data[center_index + 1],
      bitmap.data[center_index + 2],
    )]
  }
}

/// A single LED
pub fn led(name: &'static str) -> Led {
  Led::new(name)
}
