use crate::plugins::{ActionButtonEject, AutoPlunger};
use crate::prelude::*;

pub struct AutoplungerPlugin {
  plunge_lane_switch: HardwareQuery,
  autoplunge_coil: HardwareQuery,
  action_button_switch: Option<HardwareQuery>,
}

impl AutoplungerPlugin {
  /// The plugin will monitor the plunge lane and provide access to firing the autoplunger.
  /// If configured, will also allow the action button to fire the ball out of the plunge lane.
  ///
  /// ## Inputs
  /// - Command: `FirePlunger` - Fires the plunger coil if the ball is resting in the lane
  ///
  pub fn new(
    plunge_lane_switch: impl Into<HardwareQuery>,
    autoplunge_coil: impl Into<HardwareQuery>,
  ) -> Self {
    Self {
      plunge_lane_switch: plunge_lane_switch.into(),
      autoplunge_coil: autoplunge_coil.into(),
      action_button_switch: None,
    }
  }

  pub fn action_button_switch(mut self, switch: impl Into<HardwareQuery>) -> Self {
    self.action_button_switch = Some(switch.into());
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
