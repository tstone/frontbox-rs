use frontbox::prelude::*;
use frontbox::tags::StartButton;

use crate::GameManager;

/// A system to flash elements the start button and/or action button when the game is startable or player addable
pub struct StartableFlasher {
  start_button_driver: Option<HardwareQuery>,
  // TODO: action button should flash too
  // TODO: this should be more generic "startable state" that easily allows a given lamp driver/LED state when something is true
  flash_duration: Duration,
}

impl StartableFlasher {
  pub fn new() -> Self {
    Self {
      start_button_driver: Some(Q::tag::<StartButton>()),
      flash_duration: Duration::from_millis(185),
    }
  }

  pub fn start_button_driver(mut self, driver: HardwareQuery) -> Self {
    self.start_button_driver = Some(driver);
    self
  }

  fn start_btn_on(&self, ctx: &Context) {
    for driver in self.start_button_driver.get_drivers(ctx) {
      log::info!("Driver on {:?}", driver);
      ctx.activate_driver(driver.name, ActivationMode::VirtualSwitchOn);
    }
  }

  fn start_btn_off(&self, ctx: &Context) {
    for driver in self.start_button_driver.get_drivers(ctx) {
      ctx.deactivate_driver(driver.name, DeactivationMode::VirtualSwitchOff);
    }
  }
}

impl System for StartableFlasher {
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
    // for driver in self.start_button_driver.get_drivers(ctx) {
    //   ctx.configure_driver(
    //     driver.name,
    //     PulseHoldMode {
    //       trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
    //       initial_pwm_power: Power::ZERO,
    //       secondary_pwm_power: Power::FULL,
    //       ..Default::default()
    //     },
    //   );
    // }
    ctx.cue_cycling(events![On, Off], Cue::Loop(self.flash_duration));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(_) = event.downcast_ref::<On>() {
      self.start_btn_on(ctx);
    } else if let Some(_) = event.downcast_ref::<Off>() {
      self.start_btn_off(ctx);
    }
  }
}
