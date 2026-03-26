use crate::plugins::{ActionButtonEject, AutoPlunger};
use crate::prelude::*;

pub struct AutoplungerPlugin {
  plunge_lane_switch: &'static str,
  autoplunge_coil: &'static str,
  action_button: &'static str,
  led_setting: LedSetting,
}

impl AutoplungerPlugin {
  /// The plugin will monitor the plunge lane and provide access to firing the autoplunger.
  /// If configured, will also allow the action button to fire the ball out of the plunge lane.
  ///
  /// ## Inputs
  /// - Command: `FirePlunger` - Fires the plunger coil if the ball is resting in the lane
  ///
  pub fn new(
    action_button: &'static str,
    plunge_lane_switch: &'static str,
    autoplunge_coil: &'static str,
    led_setting: LedSetting,
  ) -> Self {
    Self {
      plunge_lane_switch,
      autoplunge_coil,
      action_button,
      led_setting,
    }
  }
}

impl Plugin for AutoplungerPlugin {
  fn build(&self, app: &mut App) {
    app.system(AutoPlunger::new(
      self.plunge_lane_switch,
      self.autoplunge_coil,
    ));
    app.system(ActionButtonEject::new(
      self.action_button,
      self.plunge_lane_switch,
      self.led_setting.clone(),
    ));
  }
}
