pub use crate::prelude::*;
use crate::systems::prebuilt::trough::{BallEnteredTrough, BallExitedTrough};

pub struct TroughSystem {
  pub switches: Vec<&'static str>,
  pub eject_coil: &'static str,
}

impl TroughSystem {
  /// # Arguments
  /// * `switches` - List of trough switches, in order. Index 0 is the switch nearest the exit.
  pub fn new(switches: Vec<&'static str>, eject_coil: &'static str) -> Self {
    Self {
      switches,
      eject_coil,
    }
  }

  fn on_trough_switch_closed(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      .iter()
      .last()
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      ctx.emit(BallEnteredTrough::new(self.get_occupancy(ctx)));
    }
  }

  fn on_trough_switch_opened(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      .iter()
      .next()
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      ctx.emit(BallExitedTrough::new(self.get_occupancy(ctx)));
    }
  }

  fn get_occupancy(&self, ctx: &Context) -> Vec<bool> {
    let switch_lookup = ctx.expect::<SwitchLookup>();
    self
      .switches
      .iter()
      .map(|name| switch_lookup.is_closed(name).unwrap_or(false))
      .collect()
  }
}

impl System for TroughSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    let eject_coil_name = self.eject_coil;
    ctx.register_command::<TroughEject>(move |_, ctx| {
      ctx.command(ActivateDriver::new(eject_coil_name, ActivationMode::Tap))
    });
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast::<SwitchClosed>() {
      self.on_trough_switch_closed(&e.switch.name, ctx);
    } else if let Some(e) = event.downcast::<SwitchOpened>() {
      self.on_trough_switch_opened(&e.switch.name, ctx);
    }
  }
}

pub struct TroughEject;
