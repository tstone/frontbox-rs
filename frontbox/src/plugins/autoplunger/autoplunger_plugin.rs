use crate::plugins::{ActionButtonEject, AutoPlunger};
use crate::prelude::*;

pub struct AutoplungerPlugin {
  plunge_lane_switch: HardwareSelection,
  autoplunge_coil: HardwareSelection,
  action_button_switch: Option<HardwareSelection>,
}

impl AutoplungerPlugin {
  /// The plugin will monitor the plunge lane and provide access to firing the autoplunger.
  /// If configured, will also allow the action button to fire the ball out of the plunge lane.
  ///
  /// ## Inputs
  /// - Command: `FirePlunger` - Fires the plunger coil if the ball is resting in the lane
  ///
  pub fn new(plunge_lane_switch: HardwareSelection, autoplunge_coil: HardwareSelection) -> Self {
    Self {
      plunge_lane_switch,
      autoplunge_coil,
      action_button_switch: None,
    }
  }

  pub fn action_button_switch(mut self, switch: HardwareSelection) -> Self {
    self.action_button_switch = Some(switch);
    self
  }
}

impl Plugin for AutoplungerPlugin {
  fn build(&self, app: &mut App) {
    app.system(AutoPlunger::new(
      self.plunge_lane_switch.clone(),
      self.autoplunge_coil.clone(),
    ));

    if let Some(action_button_switch) = &self.action_button_switch {
      app.system(ActionButtonEject::new(
        action_button_switch.clone(),
        self.plunge_lane_switch.clone(),
      ));
    }
  }
}
