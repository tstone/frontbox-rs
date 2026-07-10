use crate::{Tag, Illumination, Point, RenderableGeom};

#[derive(Debug, Clone)]
pub struct LedStrip {
  pub name: &'static str,
  pub tags: Vec<Box<dyn Tag>>,
  pub geom: Option<RenderableGeom>,
  pub led_count: u8,
}

impl LedStrip {
  pub fn new(name: &'static str, led_count: u8) -> Self {
    Self {
      name,
      tags: Vec::new(),
      led_count,
      geom: None,
    }
  }

  pub fn tagged(mut self, tag: impl Tag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn geom(mut self, top_left: Point, bottom_right: Point) -> Self {
    self.geom = Some(RenderableGeom::Rectangle {
      top_left,
      bottom_right,
    });
    self
  }
}

impl Illumination for LedStrip {
  fn name(&self) -> &'static str {
    self.name
  }

  fn tags(&self) -> &Vec<Box<dyn Tag>> {
    &self.tags
  }

  fn led_count(&self) -> u8 {
    self.led_count
  }

  fn geom(&self) -> Option<&RenderableGeom> {
    self.geom.as_ref()
  }

  // fn render_bitmap(&self, bitmap: &Bitmap) -> Vec<fast_protocol::Color> {
  //   match &self.geom {
  //     Some(RenderableGeom::Rectangle {
  //       top_left,
  //       bottom_right,
  //     }) => {
  //       let mut colors = Vec::new();
  //       for i in 0..self.led_count {
  //         let t = i as f32 / (self.led_count - 1) as f32;
  //         let x = top_left.0 + (bottom_right.0 - top_left.0) * t;
  //         let y = top_left.1 + (bottom_right.1 - top_left.1) * t;
  //         let index = (y.floor() as usize * bitmap.width as usize + x.floor() as usize) * 3;
  //         colors.push(fast_protocol::Color::rgb(
  //           bitmap.data[index],
  //           bitmap.data[index + 1],
  //           bitmap.data[index + 2],
  //         ));
  //       }
  //       colors
  //     }
  //     _ => vec![], // If no geometry is defined, we can't render anything meaningful
  //   }
  // }
}

/// A sequence of anonymous, addressable LEDs
pub fn led_strip(name: &'static str, led_count: u8) -> LedStrip {
  LedStrip::new(name, led_count)
}
