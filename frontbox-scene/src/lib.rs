//! # Scenes
//!
//! <div class="warning">Stability Level: Low</div>
//!
//! A [`Scene`] is layers on one timeline, each saying what it's drawn on (lights, or a display), or sounds played at set
//! times. It's what's written to a `.scene` file. The drawn layers themselves, and how they're sampled, are in
//! [`frontbox_gfx::scene`]; this crate adds where they go, and playing them.
//!
//! - [`Scene`], [`SceneLayer`] -- The definition, as written to a file
//! - [`Scenes`], [`Images`], [`Fonts`], [`Sounds`], [`ShapeGenerators`] -- What scenes are made from, each a
//!   [`Resource`](frontbox_core::prelude::Resource) of the app: files in its content folder, and what code adds
//! - [`PlayableScene`] -- A scene with values from the game bound to it: its variables, and how long it takes
//! - [`SceneSystem`] -- Plays scenes. Systems that draw a kind of layer, like `LightManager`, read each [`Playback`] from
//!   it each frame.
//! - [`ScenePlayer`] -- A scene a system turns on and off
//! - [`Sends`] -- The variables a system sends the scenes it plays, to check a scene someone wrote for it
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
//!   layers: [Lights(lights: Name("lane"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
//! )"#).unwrap();
//!
//! // plays to its end even if this system goes away
//! ctx.play_scene(&PlayableScene::completing(scene).unwrap());
//! # }
//! ```

mod moment;
mod playable_scene;
mod playback;
mod resources;
mod scene;
mod scene_player;
mod scene_system;
mod scene_system_ext;
mod sends;
mod sound_layer;
#[cfg(test)]
mod test_folder;

pub use moment::*;
pub use playable_scene::*;
pub use playback::*;
pub use resources::*;
pub use scene::*;
pub use scene_player::*;
pub use scene_system::*;
pub use scene_system_ext::*;
pub use sends::*;
pub use sound_layer::*;

use frontbox_core::prelude::*;

/// A playback played every cycle through. One that's static or repeats forever never does.
#[derive(serde::Serialize, Event, Debug, Clone, Copy)]
pub struct ScenePlaybackCompleted {
  pub playback: PlaybackId,
}

/// What a system that plays scenes uses: opening, binding variables, playing, and checking what it sends
pub mod prelude {
  pub use frontbox_gfx::animation::Curve;
  pub use frontbox_gfx::prelude::*;
  pub use frontbox_gfx::scene::{EndBehavior, Timing, Value, VariableType};

  pub use crate::{
    Completes, DisplayLayer, Fonts, Images, LightLayer, LightSurface, PlayableScene, PlaybackId,
    Scene, SceneLayer, ScenePlaybackCompleted, ScenePlayer, SceneSystem, SceneSystemExt, Scenes,
    Sends, SentVariable, ShapeGenerators, SoundLayer, Sounds,
  };
}
