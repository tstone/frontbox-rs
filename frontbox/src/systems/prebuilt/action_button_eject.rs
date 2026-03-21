use crate::prebuilt::FirePlunger;
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
  ) -> Box<Self> {
    Box::new(Self {
      action_button,
      plunge_lane_switch,
      led_setting,
      active: false,
    })
  }
}

impl System for ActionButtonEject {
  fn on_startup(&mut self, ctx: &mut Context) {
    self.active = ctx
      .expect::<SwitchLookup>()
      .is_closed(self.action_button)
      .unwrap_or(false)
  }

  fn is_active(&self, _ctx: &Context) -> bool {
    self.active
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.action_button {
        ctx.command(FirePlunger);
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
  ) -> std::collections::HashMap<&'static str, LedState> {
    let builder = LedDeclarationBuilder::new(delta_time);
    self
      .led_setting
      .add_declaration(builder, self.action_button)
      .collect()
  }
}
