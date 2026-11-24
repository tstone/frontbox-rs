//! # Scenes
//!
//! <div class="warning">Stability Level: Low. This is the start of the replacement for `canvas`.</div>
//!
//! A [`Layer`] describes a graphic and how it moves over time, as data. It's sampled rather than rendered into a buffer:
//! a device maps its points into the layer's viewport and asks for a color at each one. Where a device's points sit is up
//! to the device; see [`layout`](crate::layout).
//!
//! A scene is layers on one timeline, each saying what it's drawn on. That type lives in `frontbox-scene`; this crate
//! knows nothing about hardware, and samples one layer at a time.
//!
//! ```rust
//! # use frontbox_gfx::scene::*;
//! # use frontbox_gfx::layout::Line;
//! # use frontbox_gfx::prelude::*;
//! let layer = Layer::new(Fill::new(FillNode::Gradient {
//!   stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
//!   style: GradientStyle::Linear,
//! }));
//!
//! let line = Line::new(8);
//! let variables = Variables::new();
//! let frame = LayerFrame {
//!   time: 0.0,
//!   timing: None,
//!   playback_seed: 0,
//!   variables: &variables,
//!   defaults: no_variables(),
//!   values: no_values(),
//!   modulators: no_modulators(),
//!   assets: Assets::none(),
//!   size: Default::default(),
//!   surface: Surface::Sequence,
//!   visits: None,
//! };
//! let colors: Vec<Rgba<u8>> = line.samples().map(|s| layer.sample(&s, &frame)).collect();
//! # assert_eq!(colors[0], Rgba::red());
//! # assert_eq!(colors[7], Rgba::blue());
//! ```
//!
//! - [`Layer`] -- A fill in a shape, or a mask over the layers below it, sampled on its own, given a [`LayerFrame`]
//! - [`PreparedLayer`] -- A layer resolved once for a frame, for sampling many points. [`Layer::sample_grid`] uses one
//!   to draw a grid, such as a display's pixels.
//! - [`GraphMask`], [`Graph`] -- Limits a layer to the nodes of a graph built from the device's points, with traversals
//!   over it that [`Prop::Visit`] responds to
//! - [`Surface`] -- Whether the scene is drawn along a 1D line of points or over a 2D area
//! - [`Node`] -- The structure shared by shapes and fills: a base, plus [`Transform`]s and adjusts
//! - [`Prop`] -- A value that's fixed, read from a variable, keyframed, picked at random, moved in a wave
//!   ([`Modulated`], by one of the scene's [`Modulator`]s), or swept along a path ([`Sweep`])
//! - [`NamedValue`] -- One of a scene's named values, which a variable can point at
//! - [`Inputs`] -- The variables a layer reads, to check against what the code playing it sends
//! - [`Timing`] -- How long an animated scene runs, and how many times
//! - [`file`](mod@file) -- Reading and writing scene files as RON
//! - [`Content`], [`Assets`], [`Font`] -- The folder of scenes, images, and fonts, and the decoded images and fonts a
//!   scene draws from

mod animatable;
mod assets;
mod bounds;
mod cells;
mod color_adjust;
mod content;
mod contour;
pub mod file;
mod fill;
mod font;
mod graph;
mod graph_mask;
mod images;
mod inputs;
mod keyframes;
mod layer;
#[cfg(test)]
mod layer_tests;
mod modulated;
mod named_value;
mod node;
mod noise;
mod prop;
mod random;
mod sample;
mod scene_error;
mod shape;
mod shape_adjust;
mod surface;
mod sweep;
#[cfg(test)]
pub(crate) mod test_scene;
mod text_layout;
mod text_source;
mod timing;
mod transform;
mod value;
mod visit_response;
mod visits;

pub use animatable::*;
pub use assets::*;
pub use bounds::*;
pub use cells::CellStyle;
pub use color_adjust::*;
pub use content::*;
pub(crate) use contour::Contour;
pub use fill::*;
pub use font::*;
pub use graph::*;
pub use graph_mask::*;
pub use images::*;
pub use inputs::*;
pub use keyframes::*;
pub use layer::*;
pub use modulated::*;
pub use named_value::*;
pub use node::*;
pub use noise::NoiseStyle;
pub use prop::*;
pub use random::*;
pub use sample::*;
pub use scene_error::*;
pub use shape::*;
pub use shape_adjust::*;
pub use surface::*;
pub use sweep::*;
pub use text_layout::*;
pub use text_source::*;
pub use timing::*;
pub use transform::*;
pub use value::*;
pub use visit_response::*;
pub use visits::*;
