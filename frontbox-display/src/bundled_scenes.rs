/// The framework's screens, by the name a file in a game's content folder replaces each under. Each is in a folder for
/// the size of display it's made for.
const SCENES: [(&str, &str); 11] = [
  (
    "frontbox/128x32/attract/last_score.scene",
    include_str!("../assets/scenes/frontbox/128x32/attract/last_score.scene"),
  ),
  (
    "frontbox/128x32/attract/press_start.scene",
    include_str!("../assets/scenes/frontbox/128x32/attract/press_start.scene"),
  ),
  (
    "frontbox/128x32/menu/about.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/about.scene"),
  ),
  (
    "frontbox/128x32/menu/confirm.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/confirm.scene"),
  ),
  (
    "frontbox/128x32/menu/edit.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/edit.scene"),
  ),
  (
    "frontbox/128x32/menu/frame.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/frame.scene"),
  ),
  (
    "frontbox/128x32/menu/heading.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/heading.scene"),
  ),
  (
    "frontbox/128x32/menu/row.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/row.scene"),
  ),
  (
    "frontbox/128x32/menu/selection.scene",
    include_str!("../assets/scenes/frontbox/128x32/menu/selection.scene"),
  ),
  (
    "frontbox/128x32/player_transition.scene",
    include_str!("../assets/scenes/frontbox/128x32/player_transition.scene"),
  ),
  (
    "frontbox/128x32/points.scene",
    include_str!("../assets/scenes/frontbox/128x32/points.scene"),
  ),
];

/// The scenes this crate comes with, as each one's name and source
pub(crate) fn scenes() -> impl Iterator<Item = (&'static str, &'static str)> {
  SCENES.into_iter()
}
