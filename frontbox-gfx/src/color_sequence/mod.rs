pub mod alteration;
mod color_sequence;
pub mod fill1d;
mod fill1d_area;
mod gradient;
mod pattern;

pub use alteration::ColorSequenceAlteration;
pub use color_sequence::*;
pub use fill1d::Fill1d;
pub use fill1d_area::{Anchor1d, Fill1dArea};
pub use gradient::GradientStop;
