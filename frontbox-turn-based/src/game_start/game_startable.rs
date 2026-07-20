use frontbox::prelude::*;
use frontbox::tags::{Cabinet, StartButton};

use crate::{GameManagementExt, GameManager};

/// A system to flash elements the start button and/or action button when the game is startable or player addable
pub struct GameStartable {
  lamp_driver_name: Option<&'static str>,
  effects: Vec<LedEffect>,
  flash_duration: Duration,
}

impl GameStartable {
  pub fn new() -> Self {
    Self {
      effects: Vec::new(),
      lamp_driver_name: None,
      flash_duration: Duration::from_millis(185),
    }
  }

  pub fn flash_lamp(mut self, name: &'static str) -> Self {
    self.lamp_driver_name = Some(name);
    self
  }

  pub fn effect(mut self, effect: LedEffect) -> Self {
    self.effects.push(effect);
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
      .tag(StartButton)
  }

  fn start_btn_on(&self, ctx: &Context) {
    if let Some(name) = self.lamp_driver_name {
      ctx.activate_driver(name, ActivationMode::VirtualSwitchOn);
    }
  }

  fn start_btn_off(&self, ctx: &Context) {
    if let Some(name) = self.lamp_driver_name {
      ctx.deactivate_driver(name, DeactivationMode::VirtualSwitchOff);
    }
  }
}

impl System for GameStartable {
  fn is_active(&self, ctx: &Context) -> bool {
    // active if game is startable or player addable
    ctx
      .systems
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_deactivate(&mut self, ctx: &Context) {
    // turn off start button to make sure it's not stuck on one the system is disabled
    self.start_btn_off(ctx);
  }

  fn on_spawn(&mut self, ctx: &Context) {
    ctx.cue_cycling(events![On, Off], Cue::Loop(self.flash_duration));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(_) = event.downcast_ref::<On>() {
      self.start_btn_on(ctx);
    } else if let Some(_) = event.downcast_ref::<Off>() {
      self.start_btn_off(ctx);
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    // only apply LED effects while game isn't started (since presumably the game will have it's own LED effects)
    if !ctx.is_game_started() {
      for effect in &mut self.effects {
        effect.apply(delta, ctx);
      }
    }
  }
}
