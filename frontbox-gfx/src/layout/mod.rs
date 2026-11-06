//! # Layouts
//!
//! Where a device puts its sample points, and the stage they span. A scene doesn't know about devices; an adapter
//! picks a layout (or places points itself) and samples the scene at each point.
//!
//! - [`Line`] -- Points evenly spaced along a line, such as a strip of LEDs
//! - [`Grid`] -- Points in rows covering a stage, such as the pixels of a DMD

mod grid;
mod line;

pub use grid::*;
pub use line::*;
