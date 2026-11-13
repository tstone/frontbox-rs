use frontbox_core::prelude::*;

use crate::*;

/// Hands scenes to the [`SceneSystem`]. For a scene the system turns on and off, hold a [`ScenePlayer`] instead.
pub trait SceneSystemExt {
  /// Plays a scene to its end. Nothing owns it, so it keeps playing if this system deactivates or despawns, and playing
  /// it again while it's still going plays both.
  fn play_scene(&self, scene: &PlayableScene<Completes>);
}

impl<'a> SceneSystemExt for SystemContext<'a> {
  fn play_scene(&self, scene: &PlayableScene<Completes>) {
    match self.systems.get::<SceneSystem>() {
      Some(mut scenes) => {
        scenes.play(scene);
      }
      None => log::error!(target: "frontbox::scene", "SceneSystem not running."),
    }
  }
}
