//! # LEDs
//!
//! <div class="warning">Stability Level: Moderate-Low</div>
//!
//! Related: [Colors in Frontbox](crate::led::RgbaColor)
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
//! # use frontbox::prelude::*;
//! pub struct Example {
//!   effect: LedProgram1d
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
//! These are run through an `EffectPlayer`. The advantage of using EffectPlayer is that the effect can continue to play despite what the owning
//! system is doing or even if it is running.
//!
//! ```rust,ignore
//! // TODO: EffectPlayer / play_effect_1d
//! let effect: LedEffect1d = Rgba::red().into();
//! // fire and forget
//! ctx.play_effect_1d(effect);
//! ```
//!
//! ## LED Identification (dimensions)
//!
//! Frontbox presents LED interactions based around dimensionality.
//!
//! ### LedEffect1d
//! `LedEffect1d` describes one or more LEDs in a sequence (list), typically by name, tag, or definition. These can be used for a variety of
//! purposes including turn on lane states, flashing, breathing, pulsing, progress bars, rotational animations, and the like.
//!
//! ### LedEffect2d
//! `LedEffect2d` describes a region of the playfield on which to apply geometry or an image. The framework automatically selects the LEDs that
//! are within that region, and assigns their color. These can be used for playfield sweeps, explosions, and the like, and are often what are
//! behind attract mode style animations.
//!
//! ## LED Rendering Stack
//!
//! Generally, setting LEDs happens by way of _effects_, but understanding the stack can be useful when choosing which approach to use or building your own effects from scratch.
//!
//! - [`LedEffectPlayerNd`] - Plays temporary, one-shot style reaction effects (Requires `LedSystem`)
//! - [`LedEffectNd`] - Handles state of a sequence of LED transitions -- gradient, rotation, etc. (Requires `LedSystem`)
//! - [`LedSystem`] - Manages declarations, prioritization, and layers. Sets the final color with `Machine`. Typically not interacted with directly.
//! - [`Machine`] - Lowest level interface; Explicitly sets the state of an LED. Not recommended to interact with directly.
//!

mod alternate_resolver;
pub mod color_sequence;
mod led_declarations;
mod led_effect_modulation;
mod led_identifications;
mod led_identifications_ext;
mod led_program_1d;
mod led_system;
mod led_system_ext;
mod rgba_color;

pub(crate) use alternate_resolver::*;
pub(crate) use led_declarations::*;
pub use led_effect_modulation::*;
pub use led_identifications::*;
pub use led_identifications_ext::*;
pub use led_program_1d::*;
pub use led_system::*;
pub use led_system_ext::*;
pub use rgba_color::*;

pub use color_sequence::ColorSequence;

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum LedChannels {
  #[default]
  RGB,
  GRB,
  BRG,
  RGBW,
  GRBW,
  BRGW,
}
