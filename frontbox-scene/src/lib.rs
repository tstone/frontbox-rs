//! # Scenes
//!
//! <div class="warning">Stability Level: Low</div>
//!
//! A [`Scene`] is layers on one timeline, each saying what it's drawn on: LEDs, or a display. It's what's written to a
//! `.scene` file. The layers themselves, and how they're sampled, are in
//! [`frontbox_gfx::scene`]; this crate adds where they go, and playing them.
//!
//! - [`Scene`], [`SceneLayer`] -- The definition, as written to a file
//! - [`SceneContext`] -- A scene and how to play it: its params, images, and how long it takes
//! - [`SceneSystem`] -- Plays scenes. Systems that draw a kind of layer, like `LedSystem`, read each [`Playback`] from
//!   it each frame.
//! - [`ScenePlayer`] -- A scene a system turns on and off
//!
//! A system that uses scenes either hands one off to play to its end, or owns one with a [`ScenePlayer`]:
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_scene::*;
//! # fn example(ctx: &SystemContext) {
//! let scene = Scene::load(r#"Scene(
//!   version: 2,
//!   timing: (duration: 0.5),
//!   layers: [Leds(leds: Name("lane"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
//! )"#).unwrap();
//!
//! // plays to its end even if this system goes away
//! ctx.play_scene(&SceneContext::completing(scene).unwrap());
//! # }
//! ```

mod end_behavior;
mod moment;
mod playback;
mod scene;
mod scene_context;
mod scene_player;
mod scene_system;
mod scene_system_ext;

pub use end_behavior::*;
pub use moment::*;
pub use playback::*;
pub use scene::*;
pub use scene_context::*;
pub use scene_player::*;
pub use scene_system::*;
pub use scene_system_ext::*;

use frontbox_core::prelude::*;

/// A playback played every cycle through. One that's static or repeats forever never does.
#[derive(serde::Serialize, Event, Debug, Clone, Copy)]
pub struct ScenePlaybackCompleted {
  pub playback: PlaybackId,
}

pub mod prelude {
  pub use crate::{Completes, EndBehavior, Scene, SceneContext, SceneLayer, LedLayer, LedSurface, DisplayLayer, ScenePlayer, SceneSystem, SceneSystemExt, ScenePlaybackCompleted};
}
