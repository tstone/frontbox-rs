use std::any::Any;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::{Tag, RenderableGeom};

pub trait Illumination: DynClone + Debug + Any + Send + Sync {
  fn name(&self) -> &'static str;
  fn tags(&self) -> &Vec<Box<dyn Tag>>;
  fn geom(&self) -> Option<&RenderableGeom>;
  fn led_count(&self) -> u8;
  // fn render_bitmap(&self, bitmap: &Bitmap) -> Vec<Rgba<u8>>;
}

dyn_clone::clone_trait_object!(Illumination);
