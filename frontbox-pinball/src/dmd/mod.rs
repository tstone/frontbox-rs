mod dmd_attract_system;
mod dmd_points_system;

pub use dmd_attract_system::*;
pub use dmd_points_system::*;

use frontbox_core::prelude::Resources;
use frontbox_scene::prelude::*;

/// Scene `name` from `resources`, checked against what's sent to it, with each of its display layers at `z`. `None` if it
/// can't be opened, which is logged.
pub(crate) fn open_scene(resources: &Resources, name: &str, sends: Sends, z: i8) -> Option<Scene> {
  match Scene::open(resources, name) {
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
      log::error!(
        target: "frontbox::display",
        "Couldn't open {name}: {error}{}",
        frontbox_display::bundle_hint(name)
      );
      None
    }
  }
}

/// Resources without a content folder, with the scenes and fonts frontbox comes with
#[cfg(test)]
pub(crate) fn bundle() -> Resources {
  let mut resources = Resources::without_folder();
  frontbox_display::add_bundle(&mut resources);
  resources
}
