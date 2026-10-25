use frontbox::prelude::*;
use frontbox::provided::SwitchComboSystem;

use crate::GameManagementExt;

/// Ends the game in progress when every switch in `switches` (typically a flipper button + the start button) is held for over a second
pub fn quit_game_combo(switches: impl Into<SwitchQ>) -> SwitchComboSystem {
  SwitchComboSystem::new(switches)
    .held_for(Duration::from_millis(1250))
    .on_combo(|ctx| {
      if ctx.is_game_started() {
        log::info!(target: "frontbox::game_manager", "Quit game combo held. Ending.");
        ctx.end_game();
      }
    })
}
