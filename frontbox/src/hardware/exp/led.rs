use std::collections::HashMap;

use crate::{HardwareTag, Illumination};

#[derive(Debug, Clone)]
pub struct Led {
  name: &'static str,
  tags: Vec<Box<dyn HardwareTag>>,
  coordinates: HashMap<u8, (f32, f32)>,
}

impl Led {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      coordinates: HashMap::new(),
    }
  }

  pub fn tagged(mut self, tag: impl HardwareTag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn coords(mut self, x: f32, y: f32) -> Self {
    self.coordinates.insert(0, (x, y));
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

  fn coordinates(&self) -> &HashMap<u8, (f32, f32)> {
    &self.coordinates
  }

  fn led_count(&self) -> u8 {
    1
  }
}

/// A single LED
pub fn led(name: &'static str) -> Led {
  Led::new(name)
}
