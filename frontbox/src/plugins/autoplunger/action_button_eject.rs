use crate::plugins::AutoPlunger;
use crate::prelude::*;

/// A system to fire the auto plunger when the action button is pressed
/// Button can only be pressed when there is a ball in the plunge lane
pub struct ActionButtonEject {
  action_button: &'static str,
  plunge_lane_switch: &'static str,
  led_setting: LedSetting,
  active: bool,
}

impl ActionButtonEject {
  pub fn new(
    action_button: &'static str,
    plunge_lane_switch: &'static str,
    led_setting: LedSetting,
  ) -> Self {
    Self {
      action_button,
      plunge_lane_switch,
      led_setting,
      active: false,
    }
  }
}

impl System for ActionButtonEject {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    self.active = ctx.switches.is_closed(self.action_button).unwrap_or(false)
  }

  fn is_active(&self, _ctx: &Context, _systems: &Systems) -> bool {
    self.active
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.action_button {
        if let Some(mut autoplunger) = systems.get_mut::<AutoPlunger>() {
          autoplunger.fire(ctx, systems);
        }
      } else if e.switch.name == self.plunge_lane_switch {
        self.active = true;
      }
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      if e.switch.name == self.plunge_lane_switch {
        self.active = false;
      }
    }
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    _ctx: &Context,
    _systems: &Systems,
  ) -> std::collections::HashMap<&'static str, LedState> {
    if self.active {
      let builder = LedDeclarationBuilder::new(delta_time);
      self
        .led_setting
        .add_declaration(builder, self.action_button)
        .collect()
    } else {
      LedDeclarationBuilder::empty()
    }
  }
}
