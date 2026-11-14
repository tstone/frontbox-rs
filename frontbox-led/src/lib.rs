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
//! A system lights LEDs one of three ways, by what the LEDs are doing for the player.
//!
//! ### 1. Status
//! A system may need to set LEDs to a color to say something is true, for example "this lane is lit". That isn't a
//! scene. The system declares the color with the [`LedSystem`], and it stays until the system undeclares it or goes
//! inactive.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_led::prelude::*;
//! # mod leds {
//! #   use frontbox_core::prelude::*;
//! #   hardware_defs! {
//! #     pub LANE: LedDefinition = LedDefinition::single("lane");
//! #   }
//! # }
//! # fn example(ctx: &SystemContext) {
//! ctx.declare_leds(&leds::LANE.q(), Rgba::green());
//! # }
//! ```
//!
//! ### 2. A scene the system owns
//! What's drawn in the editor is a [`Scene`](frontbox_scene::Scene), played by the
//! [`SceneSystem`](frontbox_scene::SceneSystem). A system that turns one on and off, for example to pulse a shot while
//! it's lit, holds a [`ScenePlayer`](frontbox_scene::ScenePlayer). When the system goes inactive the scene pauses and its
//! LEDs are given up, and when the system despawns the scene goes with it.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_led::prelude::*;
//! pub struct Example {
//!   lit: bool,
//!   scene: ScenePlayer,
//! }
//!
//! impl System for Example {
//!   fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
//!     if self.lit {
//!       self.scene.start(ctx);
//!     } else {
//!       self.scene.stop(ctx);
//!     }
//!   }
//! }
//! ```
//!
//! ### 3. A scene handed off
//! Other times the LEDs indicates a _response_ to the player, such as a hit or a transition. In these cases
//! the LEDs aren't communicating a persistent state, but instead playing out a sequence or animation as a reaction. The
//! system hands the scene to the `SceneSystem` and is done with it: it plays to its end even if the system goes away.
//! Only a scene that completes can be handed off, which a
//! [`PlayableScene<Completes>`](frontbox_scene::PlayableScene) says.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_led::prelude::*;
//! # fn example(ctx: &SystemContext, hit: PlayableScene<Completes>) {
//! ctx.play_scene(&hit);
//! # }
//! ```
//!
//! ## LED Identification (dimensions)
//!
//! Each [`LedLayer`](frontbox_scene::LedLayer) of a scene says which LEDs it's on, its z, and how it lands on them with
//! its [`LedSurface`](frontbox_scene::LedSurface).
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
//! - [`SceneSystem`](frontbox_scene::SceneSystem) - Holds the scenes playing
//! - [`LedSystem`] - Draws the LED layers of those scenes with [`LedPlacement`], along with direct declarations, and
//!   resolves priority and layers. Sets the final color with `Machine`.
//! - [`Machine`](frontbox_core::machine::Machine) - Lowest level interface; Explicitly sets the state of an LED. Not recommended to interact with directly.
//!

mod alternate_resolver;
mod led_graph;
mod led_declarations;
mod led_identifications;
mod led_identifications_ext;
mod led_placement;
mod led_system;
mod led_system_ext;
#[cfg(test)]
mod scene_tests;

pub(crate) use alternate_resolver::*;
pub(crate) use led_declarations::*;
pub use led_graph::*;
pub use led_identifications::*;
pub use led_identifications_ext::*;
pub use led_placement::*;
pub use led_system::*;
pub use led_system_ext::*;

pub mod prelude {
  pub use crate::led_graph::*;
  pub use crate::led_identifications::*;
  pub use crate::led_identifications_ext::*;
  pub use crate::led_placement::*;
  pub use crate::led_system::*;
  pub use crate::led_system_ext::*;
  pub use frontbox_gfx::prelude::*;
  pub use frontbox_scene::prelude::*;
}
