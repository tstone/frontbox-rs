use frontbox::prelude::*;

use crate::GameManager;

/// A system to flash elements like the start button and/or action button when the game is startable or player addable
pub struct StartableFlasher {
  start_button_driver: Option<&'static str>,
  action_button: Option<&'static str>, // TODO: this should take the LED name and require both (combine name with LedSetting?)
  action_button_setting: Option<LedSetting>,
  flash_duration: Duration,
}

impl StartableFlasher {
  pub fn new(
    start_button_driver: Option<&'static str>,
    action_button: Option<&'static str>,
    action_button_setting: Option<LedSetting>,
  ) -> Box<Self> {
    Box::new(Self {
      start_button_driver,
      action_button,
      action_button_setting,
      flash_duration: Duration::from_millis(185),
    })
  }

  fn start_btn_on(&self, ctx: &Context, systems: &Systems) {
    if let Some(driver) = self.start_button_driver {
      systems
        .expect::<Machine>()
        .activate_driver(driver, ActivationMode::VirtualSwitchOn, ctx);
    }
  }

  fn start_btn_off(&self, ctx: &Context, systems: &Systems) {
    if let Some(driver) = self.start_button_driver {
      systems.expect::<Machine>().deactivate_driver(
        driver,
        DeactivationMode::VirtualSwitchOff,
        ctx,
      );
    }
  }
}

impl System for StartableFlasher {
  fn is_active(&self, _ctx: &Context, systems: &Systems) -> bool {
    // active if game is startable or player addable
    systems
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_deactivate(&mut self, ctx: &Context, systems: &Systems) {
    // turn off start button to make sure it's not stuck on one the system is disabled
    self.start_btn_off(ctx, systems);
  }

  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    if let Some(driver) = self.start_button_driver {
      systems.expect::<Machine>().configure_driver(
        driver,
        PulseHoldMode {
          trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
          initial_pwm_power: Power::ZERO,
          secondary_pwm_power: Power::FULL,
          ..Default::default()
        },
        ctx,
      );
    }
    ctx.cue_cycling(signals![On, Off], Cue::Loop(self.flash_duration));
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &Context, systems: &Systems) {
    if let Some(_) = cue.downcast_ref::<On>() {
      self.start_btn_on(ctx, systems);
    } else if let Some(_) = cue.downcast_ref::<Off>() {
      self.start_btn_off(ctx, systems);
    }
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    _ctx: &Context,
    _systems: &Systems,
  ) -> std::collections::HashMap<&'static str, LedState> {
    match (self.action_button, self.action_button_setting.as_mut()) {
      (Some(button), Some(setting)) => {
        let builder = LedDeclarationBuilder::new(delta_time);
        return setting.add_declaration(builder, button).collect();
      }
      _ => {}
    }

    LedDeclarationBuilder::empty()
  }
}
