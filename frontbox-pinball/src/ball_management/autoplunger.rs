use frontbox::prelude::*;

use crate::ball_management::{AutoPlungerFired, LaunchMethod, PlungeLaneSystem};

/// An autoplunger coil, and optionally the buttons that let the player fire it. Built with `BallManager::standard`.
#[derive(Clone)]
pub struct AutoPlungerConfig {
  pub(super) coil_name: &'static str,
  pub(super) launch_button: Option<SwitchQ>,
  pub(super) launch_ready_effect: Option<LedProgram1d>,
}

impl AutoPlungerConfig {
  pub(super) fn new(coil_name: &'static str) -> Self {
    Self {
      coil_name,
      launch_button: None,
      launch_ready_effect: None,
    }
  }
}

/// Fires the autoplunger coil, either when told to or when a launch button is pressed while armed
pub struct AutoPlungerSystem {
  handle: SystemHandle,
  coil_name: &'static str,
  launch_button: Option<SwitchQ>,
  launch_ready_effect: Option<LedProgram1d>,
  armed: bool,
}

impl AutoPlungerSystem {
  pub fn new(config: &AutoPlungerConfig) -> Self {
    Self {
      handle: SystemHandle::default(),
      coil_name: config.coil_name,
      launch_button: config.launch_button.clone(),
      launch_ready_effect: config.launch_ready_effect.clone(),
      armed: false,
    }
  }

  pub fn coil_definition(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name).mode(
      DriverMode::pulse_kick()
        .initial_pwm_length(HardwareValue::config(
          "Autoplunger Touch Time",
          "Duration by which the forks are brought into contact with the ball, before full launch",
          Duration::from_millis(5),
          Ranges::duration(0, 100),
        ))
        .initial_pwm_power(Power::HALF)
        .kick_length(HardwareValue::config(
          "Autoplunger Coil Launch Time",
          "Duration that the forks exert full power onto the ball (kick)",
          Duration::from_millis(8),
          Ranges::duration(5, 75),
        ))
        .build(),
    )
  }

  pub fn has_launch_button(&self) -> bool {
    self.launch_button.is_some()
  }

  pub fn is_armed(&self) -> bool {
    self.armed
  }

  /// Launch the ball resting in the plunge lane now
  pub fn launch(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    self.fire(LaunchMethod::Auto, ctx);
  }

  /// Let a launch button press launch the ball. Does nothing if no launch button is configured.
  pub fn arm_launch_button(&mut self) {
    if self.launch_button.is_none() || self.armed {
      return;
    }
    log::debug!(target: "frontbox::autoplunger", "Launch button armed");
    self.armed = true;
    if let Some(effect) = self.launch_ready_effect.as_mut() {
      effect.reset();
      effect.play();
    }
  }

  pub fn disarm_launch_button(&mut self, ctx: &ServiceContext) {
    if !self.armed {
      return;
    }
    let ctx = &ctx.for_system(self.handle);
    self.armed = false;
    if let Some(effect) = self.launch_ready_effect.as_mut() {
      effect.stop(ctx);
    }
  }

  fn fire(&mut self, method: LaunchMethod, ctx: &SystemContext) {
    log::info!(target: "frontbox::autoplunger", "Firing auto plunger ({:?})", method);
    ctx.activate_driver(self.coil_name, ActivationMode::Tap);
    ctx.emit(AutoPlungerFired { method });
  }

  fn is_ball_resting(&self, ctx: &SystemContext) -> bool {
    ctx
      .get::<PlungeLaneSystem>()
      .map(|lane| lane.is_ball_resting(ctx.into()))
      .unwrap_or(false)
  }
}

impl System for AutoPlungerSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if self.armed
      && let Some(e) = event.downcast_ref::<SwitchClosed>()
      && let Some(q) = &self.launch_button
      && q.matches(&e.switch)
    {
      // A ball still rolling (e.g. a half plunge) isn't launched; the button stays armed until it settles
      if self.is_ball_resting(ctx) {
        log::info!(target: "frontbox::autoplunger", "Launching ball via launch button");
        self.disarm_launch_button(ctx.into());
        self.fire(LaunchMethod::Button, ctx);
      }
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    if self.armed
      && let Some(effect) = self.launch_ready_effect.as_mut()
    {
      effect.apply(delta, ctx);
    }
  }
}
