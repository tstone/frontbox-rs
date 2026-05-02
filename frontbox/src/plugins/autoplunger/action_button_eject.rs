use crate::plugins::AutoPlunger;
use crate::prelude::*;

/// A system to fire the auto plunger when the action button is pressed
/// Button can only be pressed when there is a ball in the plunge lane
pub struct ActionButtonEject {
  action_button_switch: HardwareQuery,
  plunge_lane_switch: HardwareQuery,
  active: bool,
}

impl ActionButtonEject {
  pub fn new(action_button_switch: HardwareQuery, plunge_lane_switch: HardwareQuery) -> Self {
    Self {
      action_button_switch,
      plunge_lane_switch,
      active: false,
    }
  }
}

impl System for ActionButtonEject {
  fn is_active(&self, _ctx: &Context) -> bool {
    self.active
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.action_button_switch.matches_switch(&e.switch) {
        if let Some(mut autoplunger) = ctx.systems.get::<AutoPlunger>() {
          autoplunger.fire(ctx);
        }
      } else if self.plunge_lane_switch.matches_switch(&e.switch) {
        self.active = true;
      }
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      if self.plunge_lane_switch.matches_switch(&e.switch) {
        self.active = false;
      }
    }
  }
}
