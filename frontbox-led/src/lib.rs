//! # LEDs
//!
//! <div class="warning">Stability Level: Moderate-Low</div>
//!
//! Related: [Colors in Frontbox](frontbox_gfx::color::RgbaColor)
//!
//! ## Overview
//!
//! Managing LEDs comes with a few key choices. Within Frontbox there are two criteria to understand to make these choices:
//! - Usage -- What the LEDs are doing for the game and player dictates how to set them
//! - Identification -- Are the LEDs identify intentionally (1d) or by their physical location (2d)
//!
//! ## LED Usage
//!
//! An arcade machine typically uses LEDs in two ways:
//!
//! ### 1. Communication
//! A mode or system may need to specifically set LEDs to a particular state in order to communicate with the player
//! about game state, for example, signaling "hit this shot" or "extra ball available". In this case, the communicating system
//! owns and applies the effect. This is done by the system keeping the effect as an instance value of the system, applying it
//! `on_tick`, then using `play` and `stop` to control the effect.
//!
//! This also causes the owning system to owns all the declarations (see below), such that when the owning system goes inactive, the
//! corresponding LEDs can lose that effect automatically.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_led::prelude::*;
//! pub struct Example {
//!   effect: LedPlayback
//! }
//!
//! impl System for Example {
//!   fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
//!     self.effect.apply(delta, ctx);
//!   }
//! }
//! ```
//!
//! ### 2. Response
//! Other times the LEDs indicates a _response_ to the player, such as a hit effect or transition effect. In these cases the LEDs aren't
//! communicating a persistent state, but instead playing out a sequence or animation as a reaction. From the perspective of the system that
//! starts the effect, it doesn't need to babysit the effect and manually turn it on and off. Responses are temporary, one-shot type effects
//! The method of interaction here is more akin to playing a sound effect: "Just play this and I'm not worried about it".
//!
//! ```rust,ignore
//! // TODO: a way to play a scene that outlives the system starting it
//! ```
//!
//! ## LED Identification (dimensions)
//!
//! What LEDs show is an [`LedScene`], written to a `.scene` file: layers on one timeline, each saying which LEDs it's on, its
//! z, and how it lands on them with its [`Surface`].
//!
//! ### Sequence
//! A `Sequence` surface spreads over one or more LEDs in a list, typically by name, tag, or definition. These can be used for a variety of
//! purposes including turn on lane states, flashing, breathing, pulsing, progress bars, rotational animations, and the like.
//!
//! ### Area
//! An `Area` surface covers a plane, and the LEDs on it light up from where they physically are. These can be used for playfield sweeps,
//! explosions, and the like, and are often what are behind attract mode style animations.
//!
//! ### Graphs
//! A layer with a [`GraphMask`](frontbox_gfx::scene::GraphMask) only lights the LEDs of a graph built from where they
//! are, joining each to its neighbors, and runs traversals over it: a wave spreading out from an LED, a bolt along a
//! path, a random walk. Props respond to their visits with [`Prop::Visit`](frontbox_gfx::scene::Prop::Visit). On a
//! plane, LEDs more than [`GRAPH_PLANE_TOLERANCE`] off it, such as those on ramps, are left out. See [`place_graph`].
//!
//! ## LED Rendering Stack
//!
//! Generally, setting LEDs happens by way of scenes, but understanding the stack can be useful when choosing which approach to use or building your own from scratch.
//!
//! - [`LedPlayback`] - Plays an [`LedScene`] (Requires `LedSystem`)
//! - [`LedSystem`] - Manages declarations, prioritization, and layers. Sets the final color with `Machine`. Typically not interacted with directly.
//! - [`Machine`](frontbox_core::machine::Machine) - Lowest level interface; Explicitly sets the state of an LED. Not recommended to interact with directly.
//!

mod alternate_resolver;
mod end_behavior;
mod led_graph;
mod led_playback;
mod led_declarations;
mod led_scene;
mod led_identifications;
mod led_identifications_ext;
mod led_system;
mod led_system_ext;

pub(crate) use alternate_resolver::*;
pub(crate) use led_declarations::*;
pub use end_behavior::*;
pub use led_graph::*;
pub use led_playback::*;
pub use led_scene::*;
pub use led_identifications::*;
pub use led_identifications_ext::*;
pub use led_system::*;
pub use led_system_ext::*;

pub mod prelude {
  pub use crate::end_behavior::*;
  pub use crate::led_graph::*;
  pub use crate::led_playback::*;
  pub use crate::led_scene::*;
  pub use crate::led_identifications::*;
  pub use crate::led_identifications_ext::*;
  pub use crate::led_system::*;
  pub use crate::led_system_ext::*;
  pub use frontbox_gfx::prelude::*;
}
