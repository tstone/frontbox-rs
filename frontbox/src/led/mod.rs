//! # LEDs
//! 
//! <div class="warning">Stability Level: Moderate-Low</div>
//! 
//! ### Color
//! 
//! Frontbox standardizes on the the [image crate](https://crates.io/crates/image) `Rgba<u8>` color type. This color accounts for four channel colors: red, green, blue, and alpha. All rendering functions within Frontbox account for alpha channel blending.
//! 
//! Colors can be created manually.
//! 
//! ```rust
//! let red = Rgba([255, 0, 0, 255]);
//! ```
//! 
//! Or by using a handful of named colors ([see all](crate::led::RgbaColor)).
//! 
//! ```rust
//! let red = Rgba::red();
//! let cyan = Rgba::cyan();
//! ```
//! 
//! Colors can also be modified by lightness, saturation, or hue shifted.
//! 
//! ```rust
//! let c = Rgba::red()
//!   .lighten(0.4)
//!   .desaturate(0.1)
//!   .hue_shift(-15.0)
//!   .inverted();
//! ```
//! 
//! ## Rendering Forms
//! 
//! LEDs can be approached with one of two forms:
//! 
//! #### 1D
//! Setting a single LED or a sequence of LEDs (strip, group, etc.). With this form. a specific set of LEDs are referenced, and a specific color sequence is applied. This is the form typically used by game modes to communicate game state to the player.
//! 
//! 1d rendering is handled by [LedEffect] which accepts a [ColorSequence].
//! 
//! #### 2D
//! Setting a group of specially, by giving the framework graphics or effects which are then applied to LEDs that fall within that area. This is sparse canvas rendering and is used for things like attrack mode or reaction effects.
//! 
//! 2d rendering is handled by `frontbox-canvas`.
//! 
//! ## Setting LEDs
//! 
//! Setting an LED to a color can be done in multiple ways depending on the context and needs. At a high level there are two ways this is accomplished:
//! 
//! 1. [LedSystem] high-level, layered approach to multi-system LED state management (_recommended_)
//! 2. LEDs can be directly manipulated via [Machine](crate::machine::Machine) (_not recommended_, unless building a custom LED management system)
//! 
//! <div class="warning">`LedSystem` must be spawned in order to use it. The framework does not automatically start this system.</div>
//! 
//! ## Animating LEDs
//! LEDs colors can, of course, be animated -- either manually or with [LedEffects](crate::led::LedEffect).
//! 
//! ### Manually Animating LEDs
//! This approach takes more setup and manual wiring, but ends up being more flexible all overall, compared to [LedEffect]. It works by accumulating the animation _and_ re-declaring the LED on the same tick.
//! 
//! ```rust
//! pub struct AnimExample {
//!   anim: Tween<Duration, Color>
//! }
//! 
//! impl System for AnimExample {
//!   fn on_tick(&mut self, delta: Duration, ctx: &Context) {
//!     self.anim.accumulate(delta);
//! 
//!     // re-declaring the same LED will overwrite the previous declaration
//!     ctx.declare_leds(
//!       // declare the current animated value as the color of that LED
//!       &leds::EXAMPLE.q(), ColorSequence::solid(self.anim.sample())
//!     )
//!   }
//! }
//! ```
//! 
//! Any declarable attribute is animatable. For example, a common technique with pinball machines that have 3 or more LEDs for a lane is to use those LEDs to animate a pointing motion. This could be achieved by creating a group of all lane LEDs, then turning one of them on, and animation which one is lit. By giving the declaration a higher z-index, the state of the lane indicators below remains the same, but the animated effect applies "over top of" it.
//! 
//! ```rust
//! pub struct AnimExample {
//!   // notice the value being animated is a u8, not Color
//!   anim: Tween<Duration, u8>
//! }
//! 
//! impl System for AnimExample {
//!   fn on_tick(&mut self, delta: Duration, ctx: &Context) {
//!     self.anim.accumulate(delta);
//! 
//!     ctx.declare_leds(
//!       vec![&leds::LEFT_LANE_ARROW.q(), &leds::LEFT_LANE1.q(), &leds::LEFT_LANE2.q()].at_z(2),
//!       ColorSequence::pattern(self.anim.sample(), vec![Rgba::red()])
//!     )
//!   }
//! }
//! ```
//! 
//! #### Automatic Led Animation (LedEffect)
//! 
//! A simpler approach is to use an [LedEffect] that accepts a hardware query and animation, and applies it.

mod alternate_resolver;
pub mod color_sequence;
pub mod effect_systems;
mod led_declarations;
mod led_effect;
mod led_effect_modulation;
mod led_identifications;
mod led_identifications_ext;
mod led_system;
mod led_system_ext;
mod rgba_color;

pub(crate) use alternate_resolver::*;
pub(crate) use led_declarations::*;
pub use led_effect::*;
pub use led_effect_modulation::*;
pub use led_identifications::*;
pub use led_identifications_ext::*;
pub use led_system::*;
pub use led_system_ext::*;
pub use rgba_color::*;

pub use color_sequence::ColorSequence;
