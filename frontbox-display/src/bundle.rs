use frontbox_core::prelude::*;
use frontbox_scene::{Fonts, Scenes};

use crate::{bundled_fonts, bundled_scenes};

/// Adds the fonts and scenes this crate comes with to an app's resources: the pixel fonts, as
/// `frontbox/fonts/<name>.ttf`, and the screens its systems and frontbox-pinball's show, as `frontbox/<size>/...`. An app
/// that uses any of them calls this once while it's built:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # let mut app = App::new(BootConfig { config_path: None, ..Default::default() });
/// frontbox_display::add_bundle(app.resources());
/// ```
///
/// A file at the same path in the app's content folder is used in one's place, so a game can change how a framework
/// screen looks.
pub fn add_bundle(resources: &mut Resources) {
  for (name, font) in bundled_fonts::fonts() {
    resources.add::<Fonts>(name, font);
  }
  for (name, source) in bundled_scenes::scenes() {
    resources.add::<Scenes>(name, source.into());
  }
}

/// What to add to an error about scene `name` that couldn't be opened, for one that [`add_bundle`] would have added
pub fn bundle_hint(name: &str) -> &'static str {
  match name.starts_with("frontbox/") {
    true => {
      ". The scenes and fonts frontbox comes with are added with frontbox_display::add_bundle(app.resources())"
    }
    false => "",
  }
}

/// Resources without a content folder, with what this crate comes with
#[cfg(test)]
pub(crate) fn bundle() -> Resources {
  let mut resources = Resources::without_folder();
  add_bundle(&mut resources);
  resources
}

#[cfg(test)]
mod tests {
  use frontbox_scene::Scene;

  use super::*;

  #[test]
  fn every_bundled_scene_opens_with_the_bundled_fonts() {
    let resources = bundle();
    assert_eq!(resources.names::<Fonts>().len(), 13);
    for (name, _) in bundled_scenes::scenes() {
      Scene::open(&resources, name).unwrap_or_else(|error| panic!("{error}"));
    }
  }

  #[test]
  fn without_the_bundle_a_bundled_scene_says_how_to_add_it() {
    let name = "frontbox/128x32/points.scene";
    let error = Scene::open(&Resources::without_folder(), name).unwrap_err();
    assert_eq!(
      format!("{error}{}", bundle_hint(name)),
      "there's no scene \"frontbox/128x32/points.scene\": the app has no content folder. The scenes and fonts frontbox \
       comes with are added with frontbox_display::add_bundle(app.resources())"
    );
    assert_eq!(bundle_hint("dmd/mine.scene"), "");
  }
}
