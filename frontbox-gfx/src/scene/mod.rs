//! # Scenes
//!
//! <div class="warning">Stability Level: Low. This is the start of the replacement for `color_sequence` and `canvas`.</div>
//!
//! A scene describes a graphic and how it moves over time, as data that can be written to a file. It says what can be
//! seen with its [`Viewport`], and is sampled rather than rendered into a buffer: a device maps its points into the
//! viewport and asks the scene for a color at each one. Where a device's points sit is up to the device; see
//! [`layout`](crate::layout).
//!
//! ```rust
//! # use frontbox_gfx::scene::*;
//! # use frontbox_gfx::layout::Line;
//! # use frontbox_gfx::prelude::*;
//! let scene = Scene::new(Viewport::Sequence, vec![Layer::new(Paint::new(PaintNode::Gradient {
//!   stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
//!   space: ColorSpace::Oklab,
//! }))]);
//!
//! let line = Line::new(8);
//! let params = Params::new();
//! let frame = Frame { time: 0.0, params: &params, size: Default::default() };
//! let colors: Vec<Rgba<u8>> = line.samples().map(|s| scene.sample(&s, &frame)).collect();
//! # assert_eq!(colors[0], Rgba::red());
//! # assert_eq!(colors[7], Rgba::blue());
//! ```
//!
//! - [`Scene`], [`Layer`] -- Layers of paint in shapes, composited bottom to top
//! - [`Viewport`] -- What kind of surface it's seen on, either points in order or an area the device gives the size of
//! - [`Node`] -- Shapes and paints, each a base kind with [`Warp`]s going in and adjusts coming out
//! - [`Prop`] -- A value that's fixed, read from a parameter, or keyframed
//! - [`Timing`] -- How long an animated scene runs, and how many times
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
mod timing;
mod track;
mod value;
mod viewport;
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
pub use timing::*;
pub use track::*;
pub use value::*;
pub use viewport::*;
pub use warp::*;
