//! # Scenes
//!
//! <div class="warning">Stability Level: Low. This is the start of the replacement for `color_sequence` and `canvas`;
//! see the crate's PLAN.md.</div>
//!
//! A scene describes a graphic and how it moves over time, as data that can be written to a file. It's drawn on a 2d
//! stage and sampled rather than rendered into a buffer: a device decides where its points are and asks the scene for
//! a color at each one. Where a device's points sit is up to the device; see
//! [`layout`](crate::layout).
//!
//! ```rust
//! # use frontbox_gfx::scene::*;
//! # use frontbox_gfx::layout::Line;
//! # use frontbox_gfx::prelude::*;
//! let scene = Scene::new(vec![Layer::new(Paint::new(PaintNode::Gradient {
//!   stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
//!   space: ColorSpace::Oklab,
//! }))]);
//!
//! let line = Line::new(8);
//! let params = Params::new();
//! let frame = Frame { time: 0.0, stage: line.stage(), params: &params };
//! let colors: Vec<Rgba<u8>> = line.samples().map(|s| scene.sample(&s, &frame)).collect();
//! # assert_eq!(colors[0], Rgba::red());
//! # assert_eq!(colors[7], Rgba::blue());
//! ```
//!
//! - [`Scene`], [`Layer`] -- Layers of paint in shapes, composited bottom to top
//! - [`Node`] -- Shapes and paints, each a base kind with [`Warp`]s going in and adjusts coming out
//! - [`Prop`] -- A value that's fixed, read from a parameter, or keyframed
//! - [`file`] -- Reading and writing scenes as RON

mod animatable;
mod color_adjust;
pub mod file;
mod layer;
mod node;
mod paint;
mod prop;
mod bounds;
mod sample;
mod scene;
mod shape;
mod track;
mod value;
mod warp;

pub use animatable::*;
pub use color_adjust::*;
pub use layer::*;
pub use node::*;
pub use paint::*;
pub use prop::*;
pub use bounds::*;
pub use sample::*;
pub use scene::*;
pub use shape::*;
pub use track::*;
pub use value::*;
pub use warp::*;
