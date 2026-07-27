mod color_sequence;
mod extent;
pub mod fill;
mod fill_area;
mod gradient;
pub mod modification;
mod pattern;

pub use color_sequence::*;
pub use extent::*;
pub use fill::Fill;
pub use fill_area::{Anchor, FillArea};
pub use gradient::GradientStop;
pub use modification::Modification;
