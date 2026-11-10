//! # Scenes
//!
//! <div class="warning">Stability Level: Low. This is the start of the replacement for `canvas`.</div>
//!
//! A [`GfxComposition`] describes a graphic and how it moves over time, as data. It's sampled rather than rendered into a
//! buffer: a device maps its points into the scene's viewport and asks for a color at each one. Where a device's points
//! sit is up to the device; see [`layout`](crate::layout).
//!
//! A scene file holds a composition plus hardware-specific settings. Those types (`LedScene`, `DmdScene`) live in
//! `frontbox-led` and `frontbox-dmd`; this crate knows nothing about either.
//!
//! ```rust
//! # use frontbox_gfx::scene::*;
//! # use frontbox_gfx::layout::Line;
//! # use frontbox_gfx::prelude::*;
//! let composition = GfxComposition::new(vec![Layer::new(Fill::new(FillNode::Gradient {
//!   stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
//!   style: GradientStyle::Linear,
//! }))]);
//!
//! let line = Line::new(8);
//! let params = Params::new();
//! let frame = FrameContext { time: 0.0, playback_seed: 0, params: &params, images: Images::none(), size: Default::default(), surface: Surface::Sequence, visits: None };
//! let colors: Vec<Rgba<u8>> = line.samples().map(|s| composition.sample(&s, &frame)).collect();
//! # assert_eq!(colors[0], Rgba::red());
//! # assert_eq!(colors[7], Rgba::blue());
//! ```
//!
//! - [`GfxComposition`], [`Layer`] -- Layers of fill in shapes, composited bottom to top
//! - [`Mask`] -- Where a layer shows, apart from what its shape covers
//! - [`GraphMask`], [`Graph`] -- Limits a layer to the nodes of a graph built from the device's points, with traversals
//!   over it that [`Prop::Visit`] responds to
//! - [`Surface`] -- Whether the scene is drawn along a 1D line of points or over a 2D area
//! - [`Node`] -- The structure shared by shapes and fills: a base, plus [`Transform`]s and adjusts
//! - [`Prop`] -- A value that's fixed, read from a parameter, keyframed, picked at random, or swept along a path ([`Sweep`])
//! - [`Timing`] -- How long an animated scene runs, and how many times
//! - [`file`](mod@file) -- Reading and writing scene files as RON
//! - [`Content`], [`Images`] -- The folder of scenes and images, and the decoded images a scene draws from

mod animatable;
mod bounds;
mod color_adjust;
mod composition;
mod content;
pub mod file;
mod fill;
mod graph;
mod graph_mask;
mod images;
mod layer;
mod mask;
mod node;
mod prop;
mod random;
mod sample;
mod shape;
mod sweep;
#[cfg(test)]
pub(crate) mod test_scene;
mod timing;
mod track;
mod value;
mod visit_response;
mod visits;
mod surface;
mod transform;

pub use animatable::*;
pub use bounds::*;
pub use color_adjust::*;
pub use composition::*;
pub use content::*;
pub use fill::*;
pub use graph::*;
pub use graph_mask::*;
pub use images::*;
pub use layer::*;
pub use mask::*;
pub use node::*;
pub use prop::*;
pub use random::*;
pub use sample::*;
pub use shape::*;
pub use sweep::*;
pub use timing::*;
pub use track::*;
pub use value::*;
pub use visit_response::*;
pub use visits::*;
pub use surface::*;
pub use transform::*;
