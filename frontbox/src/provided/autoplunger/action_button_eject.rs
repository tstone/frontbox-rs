use crate::prelude::*;
use crate::provided::AutoPlungerSystem;

/// A system to fire the auto plunger when the action button is pressed
/// Button can only be pressed when there is a ball in the plunge lane
pub struct ActionButtonEject {
  action_button_switch: SwitchQuery,
  plunge_lane_switch: SwitchQuery,
  active: bool,
}

impl ActionButtonEject {
  pub fn new(action_button_switch: SwitchQuery, plunge_lane_switch: SwitchQuery) -> Self {
    Self {
      action_button_switch,
      plunge_lane_switch,
      active: false,
    }
  }
}

impl System for ActionButtonEject {
  fn is_active(&self, _ctx: &SystemContext) -> bool {
    self.active
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.action_button_switch.matches(&e.switch) {
        if let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
          autoplunger.fire(ctx.into());
        }
      } else if self.plunge_lane_switch.matches(&e.switch) {
        self.active = true;
      }
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      if self.plunge_lane_switch.matches(&e.switch) {
        self.active = false;
      }
    }
  }
}
