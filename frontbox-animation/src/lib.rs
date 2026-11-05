//! # Frontbox Animation
//!
//! Values that change over an accumulated amount, usually time. This crate has no dependency on the rest of
//! Frontbox; see `frontbox::animation` for how animations fit into systems.
//!
//! ```rust
//! # use std::time::Duration;
//! # use frontbox_animation::*;
//! let mut anim = Tween::new(
//!   Duration::from_secs(1),
//!   Curve::Linear,
//!   vec![0, 100],
//!   Cycle::Once
//! );
//! assert_eq!(anim.sample(), 0);
//!
//! anim.accumulate(Duration::from_millis(500));
//! assert_eq!(anim.sample(), 50);
//! ```
//!
//! ## Features
//!
//! - `image`: [`Lerp`] for `image::Rgba<u8>` and `image::RgbaImage`
//! - `fast-protocol`: [`Lerp`] for `fast_protocol::Rgb`

mod accumulator;
mod curve;
mod cycle;
mod delay;
mod hold;
mod lerp;
mod modulation;
mod modulator;
mod multi_modulator;
mod sequence;
mod tween;
mod tweenable;

pub use accumulator::*;
pub use curve::*;
pub use cycle::*;
pub use delay::*;
pub use hold::*;
pub use lerp::*;
pub use modulation::*;
pub use modulator::*;
pub use multi_modulator::*;
pub use sequence::*;
pub use tween::*;
pub use tweenable::*;

/// Describes any value that can be changed over time. More specifically, an animation is an Accumulator (something which can be marched forward with time) that returns a value.
pub trait Animation<Acc, Val>: Accumulator<Acc> + Send + Sync {
  fn sample(&self) -> Val;

  fn play(&mut self);
  fn stop(&mut self);
  fn active(&self) -> bool;
}

dyn_clone::clone_trait_object!(<A, T> Animation<A, T>);
