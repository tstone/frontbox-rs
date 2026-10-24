use frontbox::prelude::*;

use crate::ball_management::{
  AutoPlungerConfig, BallManager, PlungeLaneConfig, StandardBallManagement, TroughManagementConfig,
};

/// Configuration for `StandardBallManagement`, built with `BallManager::standard`
#[derive(Clone)]
pub struct StandardBallManagementConfig {
  pub(super) trough: TroughManagementConfig,
  pub(super) launcher: Launcher,
}

/// Where the trough ejects to, and what launches the ball from there
#[derive(Clone)]
pub(super) enum Launcher {
  /// Ejects straight into play
  Direct,
  /// Ejects into a plunge lane, which the player launches from
  PlungeLane(PlungeLaneConfig),
  /// Ejects into a plunge lane with an autoplunger
  AutoPlunger(PlungeLaneConfig, AutoPlungerConfig),
}

impl StandardBallManagementConfig {
  pub(super) fn plunge_lane(&self) -> Option<&PlungeLaneConfig> {
    match &self.launcher {
      Launcher::Direct => None,
      Launcher::PlungeLane(lane) | Launcher::AutoPlunger(lane, _) => Some(lane),
    }
  }

  pub(super) fn autoplunger(&self) -> Option<&AutoPlungerConfig> {
    match &self.launcher {
      Launcher::AutoPlunger(_, autoplunger) => Some(autoplunger),
      _ => None,
    }
  }
}

// -- Builders --
// One builder per hardware setup, so that only valid setups can be written (e.g. a launch button needs an
// autoplunger). Settings for earlier parts can be given at any later stage.

/// A trough that ejects straight into play. Add `.plunge_lane(..)` if it ejects into a plunge lane.
pub struct StandardBuilder {
  trough: TroughManagementConfig,
}

/// A trough that ejects into a plunge lane. Add `.autoplunger(..)` if the lane has one.
pub struct StandardPlungeLaneBuilder {
  trough: TroughManagementConfig,
  lane: PlungeLaneConfig,
}

/// A trough that ejects into a plunge lane with an autoplunger
pub struct StandardAutoPlungerBuilder {
  trough: TroughManagementConfig,
  lane: PlungeLaneConfig,
  autoplunger: AutoPlungerConfig,
}

impl StandardBuilder {
  pub(super) fn new(
    trough_switch_names: impl IntoIterator<Item = &'static str>,
    eject_coil_name: &'static str,
  ) -> Self {
    Self {
      trough: TroughManagementConfig {
        trough_switch_names: trough_switch_names.into_iter().collect(),
        eject_coil_name,
        ..Default::default()
      },
    }
  }

  /// The trough ejects into this plunge lane. If no eject verification switches are given, the plunge lane switch
  /// is used.
  pub fn plunge_lane(self, switch_name: &'static str) -> StandardPlungeLaneBuilder {
    StandardPlungeLaneBuilder {
      trough: self.trough,
      lane: PlungeLaneConfig::new(switch_name),
    }
  }

  pub fn build(self) -> BallManager {
    BallManager::new(StandardBallManagement::new(self.config()))
  }

  pub(super) fn config(self) -> StandardBallManagementConfig {
    StandardBallManagementConfig {
      trough: self.trough,
      launcher: Launcher::Direct,
    }
  }
}

impl StandardPlungeLaneBuilder {
  /// The plunge lane has an autoplunger
  pub fn autoplunger(self, coil_name: &'static str) -> StandardAutoPlungerBuilder {
    StandardAutoPlungerBuilder {
      trough: self.trough,
      lane: self.lane,
      autoplunger: AutoPlungerConfig::new(coil_name),
    }
  }

  pub fn build(self) -> BallManager {
    BallManager::new(StandardBallManagement::new(self.config()))
  }

  pub(super) fn config(self) -> StandardBallManagementConfig {
    StandardBallManagementConfig {
      trough: self.trough,
      launcher: Launcher::PlungeLane(self.lane),
    }
  }
}

impl StandardAutoPlungerBuilder {
  /// Buttons the player can press to launch the ball (e.g. a cabinet launch button and the action button)
  pub fn launch_button(mut self, switches: SwitchQ) -> Self {
    self.autoplunger.launch_button = Some(switches);
    self
  }

  /// An effect to play while a launch button press would launch the ball
  pub fn launch_ready_effect(mut self, effect: LedProgram1d) -> Self {
    self.autoplunger.launch_ready_effect = Some(effect.stopped());
    self
  }

  pub fn build(self) -> BallManager {
    BallManager::new(StandardBallManagement::new(self.config()))
  }

  pub(super) fn config(self) -> StandardBallManagementConfig {
    StandardBallManagementConfig {
      trough: self.trough,
      launcher: Launcher::AutoPlunger(self.lane, self.autoplunger),
    }
  }
}

macro_rules! trough_settings {
  ($($builder:ty),*) => {$(
    impl $builder {
      /// Detects a ball stuck on top of the eject position after a failed eject. It is not counted in occupancy, but
      /// while closed an eject is considered to have failed and will be retried (which typically knocks the ball loose).
      pub fn jam_switch(mut self, switch_name: &'static str) -> Self {
        self.trough.jam_switch_name = Some(switch_name);
        self
      }

      /// Switches that verify the ball has left the trough (defaults to the plunge lane switch, if there is one).
      /// Do not use anything that a ball other than the ejected ball could hit, e.g. during multiball.
      pub fn eject_verification_switches(mut self, switches: SwitchQ) -> Self {
        self.trough.eject_verification_switches = Some(switches);
        self
      }

      /// How long to wait for an eject to be verified before retrying it
      pub fn eject_verification_time(mut self, time: Duration) -> Self {
        self.trough.eject_verification_time = time;
        self
      }

      /// How many times a failed eject is retried before giving up
      pub fn eject_retries(mut self, retries: u8) -> Self {
        self.trough.eject_retries_max = retries;
        self
      }

      /// How long the trough switches must be still before occupancy is considered settled
      pub fn occupancy_settling_time(mut self, time: Duration) -> Self {
        self.trough.occupancy_settling_time = time;
        self
      }

      /// The longest occupancy waits to settle, even if the switches keep changing
      pub fn occupancy_settling_max(mut self, time: Duration) -> Self {
        self.trough.occupancy_settling_max = time;
        self
      }
    }
  )*};
}

macro_rules! plunge_lane_settings {
  ($($builder:ty),*) => {$(
    impl $builder {
      /// How long the lane switch must stay open before the ball is considered gone, as a weak plunge can roll back
      pub fn re_enter_timeout(mut self, timeout: Duration) -> Self {
        self.lane.re_enter_timeout = timeout;
        self
      }

      /// An effect to play while a ball is in the plunge lane
      pub fn ball_present_effect(mut self, effect: LedProgram1d) -> Self {
        self.lane.ball_present_effect = Some(effect);
        self
      }
    }
  )*};
}

trough_settings!(
  StandardBuilder,
  StandardPlungeLaneBuilder,
  StandardAutoPlungerBuilder
);
plunge_lane_settings!(StandardPlungeLaneBuilder, StandardAutoPlungerBuilder);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn each_stage_builds_its_launcher() {
    let direct = BallManager::standard(["t1"], "eject").config();
    assert!(matches!(direct.launcher, Launcher::Direct));

    let lane = BallManager::standard(["t1"], "eject")
      .plunge_lane("plunge")
      .config();
    assert!(matches!(lane.launcher, Launcher::PlungeLane(_)));

    let autoplunger = BallManager::standard(["t1"], "eject")
      .plunge_lane("plunge")
      .autoplunger("autoplunger")
      .launch_button(SwitchQ::name("launch"))
      .config();
    assert!(matches!(autoplunger.launcher, Launcher::AutoPlunger(..)));
    assert!(autoplunger.autoplunger().unwrap().launch_button.is_some());
  }

  #[test]
  fn earlier_settings_can_be_given_at_later_stages() {
    let config = BallManager::standard(["t1", "t2"], "eject")
      .plunge_lane("plunge")
      .autoplunger("autoplunger")
      .jam_switch("jam")
      .eject_retries(2)
      .re_enter_timeout(Duration::from_millis(500))
      .config();

    assert_eq!(config.trough.trough_switch_names, vec!["t1", "t2"]);
    assert_eq!(config.trough.eject_coil_name, "eject");
    assert_eq!(config.trough.jam_switch_name, Some("jam"));
    assert_eq!(config.trough.eject_retries_max, 2);
    assert_eq!(
      config.plunge_lane().unwrap().re_enter_timeout,
      Duration::from_millis(500)
    );
  }
}
