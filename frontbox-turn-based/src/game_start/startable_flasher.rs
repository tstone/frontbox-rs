use frontbox::prelude::*;

use crate::GameStartState;

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
}

impl System for StartableFlasher {
  fn is_active(&self, ctx: &Context) -> bool {
    ctx.is(GameStartState::GameStartable) || ctx.is(GameStartState::PlayerAddable)
    // TODO: turn off lamp driver (on_activated, on_deactivated)
  }

  fn on_startup(&mut self, ctx: &mut Context) {
    if let Some(driver) = self.start_button_driver {
      ctx.command(ConfigureDriver {
        driver,
        mode: Box::new(PulseHoldMode {
          trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
          initial_pwm_power: Power::ZERO,
          secondary_pwm_power: Power::FULL,
          ..Default::default()
        }),
      });
    }
    ctx.cue_cycling(signals![On, Off], Cue::Loop(self.flash_duration));
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &mut Context) {
    if let Some(driver) = self.start_button_driver {
      if let Some(_) = cue.downcast_ref::<On>() {
        ctx.command(ActivateDriver {
          driver,
          mode: ActivationMode::VirtualSwitchOn,
        });
      } else if let Some(_) = cue.downcast_ref::<Off>() {
        ctx.command(DeactivateDriver {
          driver,
          mode: DeactivationMode::VirtualSwitchOff,
        });
      }
    }
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    ctx: &Context,
  ) -> std::collections::HashMap<&'static str, LedState> {
    if ctx.is(GameStartState::GameStartable) {
      match (self.action_button, self.action_button_setting.as_mut()) {
        (Some(button), Some(setting)) => {
          let builder = LedDeclarationBuilder::new(delta_time);
          return setting.add_declaration(builder, button).collect();
        }
        _ => {}
      }
    }
    LedDeclarationBuilder::empty()
  }
}
