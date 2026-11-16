use frontbox_core::prelude::*;
use frontbox_core::tags::Cabinet;
use frontbox_scene::{PlayableScene, ScenePlayer};

use crate::{GamePhase, GameStarting, PlayerEntry};

/// A system to flash elements the start button and/or action button when the game is startable or player addable
pub struct GameStartable {
  lamp_driver_name: Option<&'static str>,
  /// Plays while no game is running
  scene: Option<ScenePlayer>,
  flash_duration: Duration,
}

impl GameStartable {
  pub fn new() -> Self {
    Self {
      scene: None,
      lamp_driver_name: None,
      flash_duration: Duration::from_millis(185),
    }
  }

  pub fn flash_lamp(mut self, name: &'static str) -> Self {
    self.lamp_driver_name = Some(name);
    self
  }

  pub fn scene(mut self, scene: impl Into<PlayableScene>) -> Self {
    self.scene = Some(ScenePlayer::new(scene));
    self
  }

  pub fn lamp_driver(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name)
      .mode(
        DriverMode::pulse_hold()
          .trigger_mode(DriverTriggerMode::VirtualSwitchTrue)
          .initial_pwm_power(Power::ZERO)
          .secondary_pwm_power(Power::FULL)
          .build(),
      )
      .tag(Cabinet)
  }

  fn start_btn_on(&self, ctx: &SystemContext) {
    if let Some(name) = self.lamp_driver_name {
      ctx.drivers.activate(name, ActivationMode::VirtualSwitchOn);
    }
  }

  fn start_btn_off(&self, ctx: &SystemContext) {
    if let Some(name) = self.lamp_driver_name {
      ctx.drivers.deactivate(name, DeactivationMode::VirtualSwitchOff);
    }
  }
}

impl Default for GameStartable {
  fn default() -> Self {
    Self::new()
  }
}

impl System for GameStartable {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    // active if a game can be started or a player added
    ctx.states.is(PlayerEntry::Open)
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    // turn off start button to make sure it's not stuck on one the system is disabled
    self.start_btn_off(ctx);
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.cues.start_multi(events![LampOn, LampOff], Cue::Forever(self.flash_duration));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<LampOn>() {
      self.start_btn_on(ctx);
    } else if event.is::<LampOff>() {
      self.start_btn_off(ctx);
    } else if event.is::<RoutineStarted<GameStarting>>()
      && let Some(scene) = &mut self.scene
    {
      scene.stop(ctx);
    }
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    // only played while a game hasn't started, and again once one has ended
    if ctx.states.is(GamePhase::None)
      && let Some(scene) = &mut self.scene
    {
      scene.start(ctx);
    }
  }
}

#[derive(serde::Serialize, Event)]
struct LampOn;
#[derive(serde::Serialize, Event)]
struct LampOff;
