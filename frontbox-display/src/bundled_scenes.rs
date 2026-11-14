use std::path::PathBuf;

use frontbox_gfx::scene::Content;

use crate::bundled_fonts;

/// The framework's screens, by the name a file in a game's content folder replaces each under. Each is in a folder for
/// the size of display it's made for.
const SCENES: [(&str, &str); 5] = [
  ("frontbox/128x32/menu/frame.scene", include_str!("../assets/scenes/frontbox/128x32/menu/frame.scene")),
  ("frontbox/128x32/menu/heading.scene", include_str!("../assets/scenes/frontbox/128x32/menu/heading.scene")),
  ("frontbox/128x32/menu/row.scene", include_str!("../assets/scenes/frontbox/128x32/menu/row.scene")),
  ("frontbox/128x32/player_transition.scene", include_str!("../assets/scenes/frontbox/128x32/player_transition.scene")),
  ("frontbox/128x32/points.scene", include_str!("../assets/scenes/frontbox/128x32/points.scene")),
];

/// The scenes this crate comes with, as each one's name and source, to give to
/// [`Content::with_scenes`](frontbox_gfx::scene::Content::with_scenes)
pub fn bundled_scenes() -> impl Iterator<Item = (&'static str, &'static str)> {
  SCENES.into_iter()
}

/// The content folder at `root`, or none, with the fonts and scenes this crate comes with
pub fn content_with_bundled(root: Option<PathBuf>) -> Content {
  root.map_or_else(Content::without_folder, Content::new).with_fonts(bundled_fonts()).with_scenes(bundled_scenes())
}

#[cfg(test)]
mod tests {
  use frontbox_scene::Scene;

  use super::*;

  #[test]
  fn every_bundled_scene_opens_with_the_bundled_fonts() {
    let content = content_with_bundled(None);
    for (name, _) in bundled_scenes() {
      Scene::open(&content, name).unwrap_or_else(|error| panic!("{error}"));
    }
  }
}
