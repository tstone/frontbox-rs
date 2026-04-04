use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::HardwareTag;

pub trait Illumination: DynClone + Debug + Any + Send + Sync {
  fn name(&self) -> &'static str;
  fn tags(&self) -> &Vec<Box<dyn HardwareTag>>;
  fn coordinates(&self) -> &HashMap<u8, (f32, f32)>;
  fn led_count(&self) -> u8;
}

dyn_clone::clone_trait_object!(Illumination);

#[derive(Debug, Clone)]
pub struct ResolvedIllumination {
  pub name: &'static str,
  pub tags: Vec<Box<dyn HardwareTag>>,
  pub coordinates: HashMap<u8, (f32, f32)>,
  pub global_indexes: Vec<u16>,
  pub source: Box<dyn Illumination>,
}

impl ResolvedIllumination {
  pub fn is<T: Illumination + 'static>(&self) -> bool {
    self.source.as_any().is::<T>()
  }
}
