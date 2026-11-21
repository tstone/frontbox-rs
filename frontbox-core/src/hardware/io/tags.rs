use dyn_clone::DynClone;
use frontbox_derive::Tag;
use std::any::Any;
use std::fmt::Debug;

/// Targets are a data-less struct that acts as a typed classifier. Tags must always implement `Tag` (derivable).
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// #[derive(Tag)]
/// pub struct Example;
///
/// #[derive(Tag)]
/// pub struct Rainbow;
/// ```
pub trait Tag: Debug + DynClone + Send + Sync + Any {
  fn as_any(&self) -> &dyn Any;
  fn tag_name(&self) -> &'static str;
}

dyn_clone::clone_trait_object!(Tag);

impl serde::Serialize for dyn Tag {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.tag_name())
  }
}

#[derive(Tag)]
pub struct Button;
