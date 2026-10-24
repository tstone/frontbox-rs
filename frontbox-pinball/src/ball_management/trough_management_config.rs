use std::time::Duration;

use frontbox::prelude::*;

// TODO: would be nice to make some of these operator config (eject retries, max retries, etc.)

#[derive(Debug, Clone)]
pub struct TroughManagementConfig {
  // ordered nearest the eject to farthest
  pub(super) trough_switch_names: Vec<&'static str>,
  // If given, detects a ball stuck on top of the eject position after a failed eject. It is not counted in occupancy,
  // but while closed an eject is considered to have failed and will be retried (which typically knocks the ball loose).
  pub(super) jam_switch_name: Option<&'static str>,
  pub(super) eject_coil_name: &'static str,
  pub(super) eject_verification_time: Duration,
  /// If given, will be used to verify the ball has exited the trough (e.g. the plunge lane switch)
  /// Do not set to anything that a ball other than the ejected ball would be hitting in the case of a multiball
  /// Do not set if the game design allows the ball to intentionally re-enter the plunge lane
  /// If not given, it will be ambiguous for the trough if an eject failed or if a ball re-entered. If not set, consider lowering eject verification time.
  pub(super) eject_verification_switches: Option<SwitchQ>,
  pub(super) eject_retries_max: u8,
  pub(super) occupancy_settling_time: Duration,
  pub(super) occupancy_settling_max: Duration,
}

impl Default for TroughManagementConfig {
  fn default() -> Self {
    Self {
      trough_switch_names: Vec::new(),
      jam_switch_name: None,
      eject_verification_switches: None,
      eject_coil_name: "",
      eject_verification_time: Duration::from_millis(1000),
      eject_retries_max: 4,
      occupancy_settling_time: Duration::from_millis(300),
      occupancy_settling_max: Duration::from_millis(1500),
    }
  }
}
