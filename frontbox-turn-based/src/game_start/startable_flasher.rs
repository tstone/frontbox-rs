use frontbox::prelude::*;
use frontbox::tags::StartButton;

use crate::GameManager;

/// A system to flash elements like the start button and/or action button when the game is startable or player addable
pub struct StartableFlasher {
  start_button_driver: Option<HardwareSelection>,
  flash_duration: Duration,
}

impl StartableFlasher {
  pub fn new() -> Self {
    Self {
      start_button_driver: Some(HardwareSelection::tag::<StartButton>()),
      flash_duration: Duration::from_millis(185),
    }
  }

  pub fn start_button_driver(mut self, driver: HardwareSelection) -> Self {
    self.start_button_driver = Some(driver);
    self
  }

  fn start_btn_on(&self, ctx: &Context, systems: &Systems) {
    for driver in self.start_button_driver.get_drivers(ctx) {
      log::info!("Driver on {:?}", driver);
      systems.expect::<Machine>().activate_driver(
        driver.name,
        ActivationMode::VirtualSwitchOn,
        ctx,
      );
    }
  }

  fn start_btn_off(&self, ctx: &Context, systems: &Systems) {
    for driver in self.start_button_driver.get_drivers(ctx) {
      systems.expect::<Machine>().deactivate_driver(
        driver.name,
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
    for driver in self.start_button_driver.get_drivers(ctx) {
      systems.expect::<Machine>().configure_driver(
        driver.name,
        PulseHoldMode {
          trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
          initial_pwm_power: Power::ZERO,
          secondary_pwm_power: Power::FULL,
          ..Default::default()
        },
        ctx,
      );
    }
    ctx.cue_cycling(events![On, Off], Cue::Loop(self.flash_duration));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    if let Some(_) = event.downcast_ref::<On>() {
      self.start_btn_on(ctx, systems);
    } else if let Some(_) = event.downcast_ref::<Off>() {
      self.start_btn_off(ctx, systems);
    }
  }
}
