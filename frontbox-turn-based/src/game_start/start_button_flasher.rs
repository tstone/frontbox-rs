use frontbox::prelude::*;

use crate::GameStartState;

/// A system to flash the start button when the game is startable or player addable
pub struct StartButtonFlasher {
  start_button_driver: &'static str,
  flash_duration: Duration,
  on: bool,
}

impl StartButtonFlasher {
  pub fn new(start_button_driver: &'static str) -> Box<Self> {
    Box::new(Self {
      start_button_driver,
      flash_duration: Duration::from_millis(185),
      on: false,
    })
  }

  pub fn custom(start_button_driver: &'static str, flash_hz: f32) -> Box<Self> {
    Box::new(Self {
      start_button_driver,
      // convert hz to duration then talk half for on/off time
      flash_duration: Duration::from_secs_f32(1.0 / flash_hz / 2.0),
      on: false,
    })
  }
}

impl System for StartButtonFlasher {
  fn is_active(&self, ctx: &Context) -> bool {
    ctx.is(GameStartState::GameStartable) || ctx.is(GameStartState::PlayerAddable)
  }

  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.command(ConfigureDriver {
      driver: self.start_button_driver,
      mode: Box::new(PulseHoldMode {
        trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
        initial_pwm_power: Power::ZERO,
        secondary_pwm_power: Power::FULL,
        ..Default::default()
      }),
    });
    ctx.set_timer("flash", self.flash_duration, TimerMode::Repeating);
  }

  fn on_timer(&mut self, _timer_id: &str, ctx: &mut Context) {
    self.on = !self.on;
    if self.on {
      ctx.command(ActivateDriver {
        driver: self.start_button_driver,
        mode: ActivationMode::VirtualSwitchOn,
      });
    } else {
      ctx.command(DeactivateDriver {
        driver: self.start_button_driver,
        mode: DeactivationMode::VirtualSwitchOff,
      });
    }
  }
}
