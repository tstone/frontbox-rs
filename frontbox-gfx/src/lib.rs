//! # Frontbox Graphics
//!
//! Colors, animation, and rendering that don't depend on the rest of Frontbox: nothing here knows about systems,
//! hardware, or LEDs. Crates that connect graphics to a machine (like `frontbox-led`) build on top of this one.
//!
//! - [`animation`] -- Values that change over an accumulated amount, usually time
//! - [`color`] -- Named colors and color adjustments for `Rgba<u8>`
//! - [`canvas`] -- Two dimensional rendering: layers composited onto an image
//! - [`scene`] -- Graphics as data, sampled at any point: the replacement for `canvas`
//! - [`layout`] -- Where a device's sample points sit in a scene's viewport
//! - [`extent`] -- Positions and lengths that are either absolute or relative

pub mod animation;
pub mod canvas;
pub mod color;
pub mod scene;
pub mod extent;
pub mod layout;

pub mod prelude {
  pub use crate::animation::Cycle;
  pub use crate::color::*;
  pub use crate::extent::*;

  // re-exports
  pub use image::Rgba;
}
