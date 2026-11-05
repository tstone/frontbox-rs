use frontbox_core::prelude::*;

use crate::{GameEndReason, GameManagementExt, GamePhase, SlamTilted, metrics};

/// Ends the game at once when the slam tilt switch closes. A slam tilt is physical abuse of the machine (it was kicked,
/// or the coin door slammed), so nothing of the game is kept: no turn ending, no extra balls.
///
/// Each one is logged as a warning, counted (`metrics::SLAM_TILTS`), and emits `SlamTilted`. During a game, the game
/// ends with `GameEndReason::SlamTilted`, so `GameEnding` participants can skip (high-score entry, match). Outside a
/// game, it's only logged and counted.
///
/// ## Setup
/// ```rust,ignore
/// // at the root
/// SlamTiltSystem::new(&switches::SLAM_TILT)
/// ```
pub struct SlamTiltSystem {
  switch: SwitchQ,
}

impl SlamTiltSystem {
  pub fn new(switch: impl Into<SwitchQ>) -> Self {
    Self {
      switch: switch.into(),
    }
  }
}

impl System for SlamTiltSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>() else {
      return;
    };
    if !self.switch.matches(switch) {
      return;
    }

    let in_game = !ctx.states.is(GamePhase::None);
    log::warn!(target: "frontbox_core::tilt", "Slam tilt ({}), {}", switch.name, if in_game { "ending the game" } else { "outside a game" });
    ctx.metrics.inc(&metrics::SLAM_TILTS);
    ctx.emit(SlamTilted);
    if in_game {
      ctx.quit_game(GameEndReason::SlamTilted);
    }
  }
}
