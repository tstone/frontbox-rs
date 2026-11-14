//! # Scenes
//!
//! <div class="warning">Stability Level: Low</div>
//!
//! A [`Scene`] is layers on one timeline, each saying what it's drawn on (LEDs, or a display), or sounds played at set
//! times. It's what's written to a `.scene` file. The drawn layers themselves, and how they're sampled, are in
//! [`frontbox_gfx::scene`]; this crate adds where they go, and playing them.
//!
//! - [`Scene`], [`SceneLayer`] -- The definition, as written to a file
//! - [`PlayableScene`] -- A scene with values from the game bound to it: its params, and how long it takes
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
//! ctx.play_scene(&PlayableScene::completing(scene).unwrap());
//! # }
//! ```

mod moment;
mod playable_scene;
mod playback;
mod scene;
mod scene_player;
mod scene_system;
mod scene_system_ext;
mod sound_layer;

pub use moment::*;
pub use playable_scene::*;
pub use playback::*;
pub use scene::*;
pub use scene_player::*;
pub use scene_system::*;
pub use scene_system_ext::*;
pub use sound_layer::*;

use frontbox_core::prelude::*;

/// A playback played every cycle through. One that's static or repeats forever never does.
#[derive(serde::Serialize, Event, Debug, Clone, Copy)]
pub struct ScenePlaybackCompleted {
  pub playback: PlaybackId,
}

pub mod prelude {
  pub use frontbox_gfx::scene::EndBehavior;

  pub use crate::{Completes, Scene, PlayableScene, SceneLayer, LedLayer, LedSurface, DisplayLayer, SoundLayer, ScenePlayer, SceneSystem, SceneSystemExt, ScenePlaybackCompleted};
}
