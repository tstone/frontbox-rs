//! # Animation
//!
//! The pieces scenes animate with: [`Curve`]s for easing between values, [`Cycle`] for how many times something plays,
//! and [`Lerp`] for values that can be eased between.

mod curve;
mod cycle;
mod lerp;

pub use curve::*;
pub use cycle::*;
pub use lerp::*;
