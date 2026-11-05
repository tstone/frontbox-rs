//! # Layouts
//!
//! Where a device puts its sample points, and the stage they span. A scene doesn't know about devices; an adapter
//! picks a layout (or places points itself) and samples the scene at each point.
//!
//! - [`Line`] -- Points evenly spaced along a line, such as a strip of LEDs

mod line;

pub use line::*;
