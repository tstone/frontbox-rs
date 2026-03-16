use std::sync::{Arc, Mutex};

pub use crate::prelude::*;
use crate::systems::prebuilt::trough::{BallEnteredTrough, BallExitedTrough};

pub struct TroughSystem {
  pub switches: Vec<&'static str>,
  pub eject_coil: &'static str,
  pub expected_occupancy: Arc<Mutex<usize>>,
}

impl TroughSystem {
  /// # Arguments
  /// * `switches` - List of trough switches, in order. Index 0 is the switch nearest the exit.
  pub fn new(switches: Vec<&'static str>, eject_coil: &'static str) -> Self {
    Self {
      expected_occupancy: Arc::new(Mutex::new(switches.len())),
      switches,
      eject_coil,
    }
  }

  fn on_trough_switch_closed(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      // only look at the last switch (nearest the exit) for occupancy changes, since that's the only one that should trigger a change in occupancy. This allows for things like physical ball locks to be used with the trough without causing issues with occupancy calculations.
      .get(*self.expected_occupancy.lock().unwrap())
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      ctx.emit(BallEnteredTrough::new(self.get_occupancy(ctx)));
    }
  }

  fn on_trough_switch_opened(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      .get(0) // only look at the first switch (nearest the eject point)
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
    // TODO: should this set debounce settings? or just have a recommendation in the comments

    let eject_coil_name = self.eject_coil;
    ctx.register_command::<TroughEject>(move |_, _, ctx| {
      ctx.command(ActivateDriver::new(eject_coil_name, ActivationMode::Tap))
    });

    let max_occupancy = self.switches.len();
    let expected_occupancy = self.expected_occupancy.clone();
    ctx.register_command::<BallAddedToPlay>(move |_, _, _| {
      let occupancy = expected_occupancy.lock().unwrap();
      if *occupancy < max_occupancy {
        *expected_occupancy.lock().unwrap() += 1;
      }
    });

    let expected_occupancy = self.expected_occupancy.clone();
    ctx.register_command::<BallRemovedFromPlay>(move |_, _, _| {
      let _ = expected_occupancy.lock().unwrap().saturating_sub(1);
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

/// This command causes the trough to expect one less ball in it's occupancy calculations. This is typically called in situations where something like a physical ball lock is holding onto a ball that should no longer be expected in the trough.
pub struct BallRemovedFromPlay;
/// This command causes the trough to expect one more ball in it's occupancy calculations. This is typically called in situations where a ball is added back into play, such as when a ball is released from a physical lock.
pub struct BallAddedToPlay;
