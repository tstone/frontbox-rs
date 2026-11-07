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
//! let scene = Scene::new(Viewport::Sequence, vec![Layer::new(Fill::new(FillNode::Gradient {
//!   stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
//!   space: ColorSpace::Oklab,
//!   style: GradientStyle::Linear,
//! }))]);
//!
//! let line = Line::new(8);
//! let params = Params::new();
//! let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: Default::default() };
//! let colors: Vec<Rgba<u8>> = line.samples().map(|s| scene.sample(&s, &frame)).collect();
//! # assert_eq!(colors[0], Rgba::red());
//! # assert_eq!(colors[7], Rgba::blue());
//! ```
//!
//! - [`Scene`], [`Layer`] -- Layers of fill in shapes, composited bottom to top
//! - [`Mask`] -- Where a layer shows, apart from what its shape covers
//! - [`Viewport`] -- What kind of surface it's seen on, either points in order or an area the device gives the size of
//! - [`Node`] -- Shapes and fills, each a base kind with [`Transform`]s going in and adjusts coming out
//! - [`Prop`] -- A value that's fixed, read from a parameter, keyframed, picked at random, or swept along a path ([`Sweep`])
//! - [`Timing`] -- How long an animated scene runs, and how many times
//! - [`file`] -- Reading and writing scenes as RON
//! - [`Content`], [`Images`] -- The folder of what scenes use, and the images it decodes for one to play with

mod animatable;
mod bounds;
mod color_adjust;
mod content;
pub mod file;
mod fill;
mod images;
mod layer;
mod mask;
mod node;
mod prop;
mod random;
mod sample;
mod scene;
mod shape;
mod sweep;
mod timing;
mod track;
mod value;
mod viewport;
mod transform;

pub use animatable::*;
pub use bounds::*;
pub use color_adjust::*;
pub use content::*;
pub use fill::*;
pub use images::*;
pub use layer::*;
pub use mask::*;
pub use node::*;
pub use prop::*;
pub use random::*;
pub use sample::*;
pub use scene::*;
pub use shape::*;
pub use sweep::*;
pub use timing::*;
pub use track::*;
pub use value::*;
pub use viewport::*;
pub use transform::*;
