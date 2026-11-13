use frontbox_core::prelude::*;

use crate::coin_door::CoinDoor;

/// Reads the coin door switch into the `CoinDoor` state, for systems that act differently while an operator has the
/// door open. Ball search doesn't run, and with `configs::IGNORE_DRAINS_WITH_COIN_DOOR_OPEN` a drain is put back into
/// play rather than counting (no ball save, no end of turn), so an operator can lift the glass and take out a stuck
/// ball.
///
/// ## Setup
/// ```rust,ignore
/// // at the root
/// CoinDoorSystem::new(coin_door::OPEN_SWITCH.name)
/// ```
pub struct CoinDoorSystem {
  /// Closed while the door is shut
  switch: &'static str,
}

impl CoinDoorSystem {
  pub fn new(switch: &'static str) -> Self {
    Self { switch }
  }
}

impl System for CoinDoorSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    if ctx.switches.is_open(self.switch).unwrap_or(false) {
      ctx.states.set(CoinDoor::Open);
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(SwitchOpened { switch }) = event.downcast_ref::<SwitchOpened>()
      && switch.name == self.switch
    {
      log::info!(target: "frontbox::coin_door", "Coin door opened");
      ctx.states.set(CoinDoor::Open);
    } else if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>()
      && switch.name == self.switch
    {
      log::info!(target: "frontbox::coin_door", "Coin door closed");
      ctx.states.set(CoinDoor::Closed);
    }
  }
}
