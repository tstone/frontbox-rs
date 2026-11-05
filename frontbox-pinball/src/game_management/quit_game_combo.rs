use frontbox_core::prelude::*;
use frontbox_core::provided::SwitchComboSystem;

use crate::{GameEndReason, GameManagementExt, GamePhase};

/// Quits the game in progress when every switch in `switches` (typically a flipper button + the start button) is held for over a second
pub fn quit_game_combo(switches: impl Into<SwitchQ>) -> SwitchComboSystem {
  SwitchComboSystem::new(switches)
    .held_for(Duration::from_millis(1250))
    .on_combo(|ctx| {
      if !ctx.states.is(GamePhase::None) {
        log::info!(target: "frontbox_core::game_manager", "Quit game combo held");
        ctx.quit_game(GameEndReason::Quit);
      }
    })
}
