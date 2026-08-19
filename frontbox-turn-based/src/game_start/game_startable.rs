use frontbox::prelude::*;
use frontbox::tags::Cabinet;

use crate::{GameEnded, GameManagementExt, GameManager, GameStarted};

/// A system to flash elements the start button and/or action button when the game is startable or player addable
pub struct GameStartable {
  lamp_driver_name: Option<&'static str>,
  led_programs: Vec<LedProgram>,
  flash_duration: Duration,
}

impl GameStartable {
  pub fn new() -> Self {
    Self {
      led_programs: Vec::new(),
      lamp_driver_name: None,
      flash_duration: Duration::from_millis(185),
    }
  }

  pub fn flash_lamp(mut self, name: &'static str) -> Self {
    self.lamp_driver_name = Some(name);
    self
  }

  pub fn effect(mut self, effect: LedProgram) -> Self {
    self.led_programs.push(effect);
    self
  }

  pub fn lamp_driver(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name)
      .mode(PulseHoldMode {
        trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
        initial_pwm_power: HardwareValue::fixed(Power::ZERO),
        secondary_pwm_power: HardwareValue::fixed(Power::FULL),
        ..Default::default()
      })
      .tag(Cabinet)
  }

  fn start_btn_on(&self, ctx: &SystemContext) {
    if let Some(name) = self.lamp_driver_name {
      ctx.activate_driver(name, ActivationMode::VirtualSwitchOn);
    }
  }

  fn start_btn_off(&self, ctx: &SystemContext) {
    if let Some(name) = self.lamp_driver_name {
      ctx.deactivate_driver(name, DeactivationMode::VirtualSwitchOff);
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
    // active if game is startable or player addable
    ctx
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    // turn off start button to make sure it's not stuck on one the system is disabled
    self.start_btn_off(ctx);
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    ctx.cue_cycling(events![LampOn, LampOff], Cue::Loop(self.flash_duration));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<LampOn>() {
      self.start_btn_on(ctx);
    } else if event.is::<LampOff>() {
      self.start_btn_off(ctx);
    } else if event.is::<GameStarted>() {
      for effect in &mut self.led_programs {
        effect.stop(ctx);
      }
    } else if event.is::<GameEnded>() {
      for effect in &mut self.led_programs {
        effect.play();
      }
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    // only apply effects if a game hasn't started
    if !ctx.is_game_started() {
      for effect in &mut self.led_programs {
        effect.apply(delta, ctx);
      }
    }
  }
}

#[derive(serde::Serialize, Event)]
struct LampOn;
#[derive(serde::Serialize, Event)]
struct LampOff;
