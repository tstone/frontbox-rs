//! # Layouts
//!
//! How a device's points map into a scene's viewport. A scene doesn't know about devices; an adapter
//! picks a layout (or places points itself) and samples the scene at each point.
//!
//! - [`Line`] -- Points evenly spaced along a line, such as a strip of LEDs
//! - [`Grid`] -- Points in rows fit to an area, such as the pixels of a DMD

mod grid;
mod line;

pub use grid::*;
pub use line::*;
