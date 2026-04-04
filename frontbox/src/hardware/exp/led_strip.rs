use std::collections::HashMap;

use crate::{HardwareTag, Illumination};

#[derive(Debug)]
pub struct LedStrip {
  pub name: &'static str,
  pub tags: Vec<Box<dyn HardwareTag>>,
  pub coordinates: HashMap<u8, (f32, f32)>,
  pub led_count: u8,
}

impl LedStrip {
  pub fn new(name: &'static str, led_count: u8) -> Self {
    Self {
      name,
      tags: Vec::new(),
      coordinates: HashMap::new(),
      led_count,
    }
  }

  pub fn tagged(mut self, tag: impl HardwareTag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn coords(mut self, index: u8, x: f32, y: f32) -> Self {
    self.coordinates.insert(index, (x, y));
    self
  }
}

impl Illumination for LedStrip {
  fn name(&self) -> &'static str {
    self.name
  }

  fn tags(&self) -> &Vec<Box<dyn HardwareTag>> {
    &self.tags
  }

  fn coordinates(&self) -> &HashMap<u8, (f32, f32)> {
    &self.coordinates
  }

  fn led_count(&self) -> u8 {
    self.led_count
  }
}

/// A sequence of anonymous, addressable LEDs
pub fn led_strip(name: &'static str, led_count: u8) -> LedStrip {
  LedStrip::new(name, led_count)
}
