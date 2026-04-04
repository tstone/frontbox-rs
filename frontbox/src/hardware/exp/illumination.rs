use std::collections::HashMap;
use std::fmt::Debug;

use crate::HardwareTag;

pub trait Illumination: Debug {
  fn name(&self) -> &'static str;
  fn tags(&self) -> &Vec<Box<dyn HardwareTag>>;
  fn coordinates(&self) -> &HashMap<u8, (f32, f32)>;
  fn led_count(&self) -> u8;
}

#[derive(Debug, Clone)]
pub struct ResolvedIllumination {
  pub name: &'static str,
  pub tags: Vec<Box<dyn HardwareTag>>,
  pub coordinates: HashMap<u8, (f32, f32)>,
  pub global_indexes: Vec<u16>,
}
