//! # Frontbox Graphics
//!
//! Colors, animation, and rendering that don't depend on the rest of Frontbox: nothing here knows about systems,
//! hardware, or LEDs. Crates that connect graphics to a machine (like `frontbox-lighting`) build on top of this one.
//!
//! - [`animation`] -- Easing curves, cycles, and values that can be eased between
//! - [`color`] -- Named colors and color adjustments for `Rgba<u8>`
//! - [`scene`] -- Graphics as data, sampled at any point
//! - [`layout`] -- Where a device's sample points sit in a scene's viewport

pub mod animation;
pub mod color;
pub mod scene;
pub mod layout;

pub mod prelude {
  pub use crate::animation::Cycle;
  pub use crate::color::*;

  // re-exports
  pub use image::Rgba;
}
