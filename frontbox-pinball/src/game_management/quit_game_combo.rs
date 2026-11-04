use frontbox::prelude::*;
use frontbox::provided::SwitchComboSystem;

use crate::{GameEndReason, GameManagementExt, GamePhase};

/// Quits the game in progress when every switch in `switches` (typically a flipper button + the start button) is held for over a second
pub fn quit_game_combo(switches: impl Into<SwitchQ>) -> SwitchComboSystem {
  SwitchComboSystem::new(switches)
    .held_for(Duration::from_millis(1250))
    .on_combo(|ctx| {
      if !ctx.in_state(GamePhase::None) {
        log::info!(target: "frontbox::game_manager", "Quit game combo held");
        ctx.quit_game(GameEndReason::Quit);
      }
    })
}
