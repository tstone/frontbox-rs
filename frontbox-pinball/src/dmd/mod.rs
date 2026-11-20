mod dmd_attract_system;
mod dmd_points_system;

pub use dmd_attract_system::*;
pub use dmd_points_system::*;

use frontbox_scene::prelude::*;

/// Scene `name` from `content`, checked against what's sent to it, with each of its display layers at `z`. `None` if it
/// can't be opened, which is logged.
pub(crate) fn open_scene(content: &Content, name: &str, sends: Sends, z: i8) -> Option<Scene> {
  match Scene::open(content, name) {
    Ok(mut scene) => {
      sends.log_mismatches(name, &scene);
      scene
        .layers
        .iter_mut()
        .filter_map(SceneLayer::as_display_mut)
        .for_each(|layer| layer.z = z);
      Some(scene)
    }
    Err(error) => {
      log::error!(target: "frontbox::display", "Couldn't open {name}: {error}");
      None
    }
  }
}
