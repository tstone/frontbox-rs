//! # Animation
//!
//! <div class="warning">Stability Level: Moderate</div>
//!
//! Animations are a fundamental part of any arcade machine and especially to pinball. Whereas a `Cue` is about an event in time that a system handles, an animation is about a value that changes over time (though not necessarily bound to time). It's useful to establish first what exactly an animation is, before demonstrating how to use it.
//!
//! Animations describe "how does this value change over an accumulated amount?" Usually the thing being accumulated is time.
//!
//! ```rust
//! # use std::time::Duration;
//! # use frontbox_gfx::animation::*;
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
//! This example describes how a value will start at `0` and end up at `100` over the duration of 1 second. The current value of the animation can be read by sampling it (`.sample()`). Calling `accumulate` causes time to march forward. Sampling the value of changed time will yield a new value.
//!
//! #### Ticking Forward
//!
//! Animations are actually built on a lower level trait called a `Accumulator`. Accumulator are, as the name implies, accumulators of values. When used with `Duration` they accumulate time.
//!
//! ```rust
//! # use std::time::Duration;
//! # use frontbox_gfx::animation::*;
//! # let mut acc = Tween::new(Duration::from_secs(1), Curve::Linear, vec![0, 100], Cycle::Once);
//! acc.accumulate(Duration::from_millis(100));
//! assert!(!acc.is_complete());
//!
//! acc.reset();
//! ```
//!
//! In Frontbox, systems have an `on_tick` handler, invoked by the framework, that marches forward based on the framework frequency much like all game frameworks. An animation owned by a system is moved forward there, by passing along the tick's `delta`:
//!
//! ```rust
//! # use std::time::Duration;
//! # use frontbox_gfx::animation::*;
//! struct Example {
//!   anim: Tween<Duration, i32>,
//! }
//!
//! impl Example {
//!   // called from `System::on_tick`
//!   fn on_tick(&mut self, delta: Duration) {
//!     self.anim.accumulate(delta);
//!   }
//! }
//! ```
//!
//! #### Accumulation
//!
//! While in the example above the animation was accumulating time by way of `Duration`, it's possible to accumulate anything that is, well, accumulatable. There are a few trait restrictions, like it must have a default value and be comparable (`PartialOrd`), summable, etc. but beyond that any accumulatable value can be accumulated.
//!
//! This means that animations work, not just on time, by for integers that represent hit counts or switch counts. For example, to change the color of LED based on how many time a spinner has spun, an animation can be used for this.
//!
//! ```rust
//! # use frontbox_gfx::animation::*;
//! # use frontbox_gfx::prelude::*;
//! // Require 100 hits, animating a from yellow to red
//! let mut anim = Tween::new(
//!   100, // target
//!   Curve::Linear,
//!   vec![Rgba::yellow(), Rgba::red()],
//!   Cycle::Once
//! );
//!
//! // each time the spinner switch closes
//! let result = anim.accumulate(1);
//! if result.completed_cycle {
//!   // do something
//! }
//!
//! // elsewhere, such as when rendering, the animation value can be used to set the LED color
//! let color = anim.sample();
//! # let _ = color;
//! ```

mod accumulator;
mod curve;
mod cycle;
mod delay;
mod hold;
mod lerp;
mod sequence;
mod tween;
mod tweenable;

pub use accumulator::*;
pub use curve::*;
pub use cycle::*;
pub use delay::*;
pub use hold::*;
pub use lerp::*;
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
