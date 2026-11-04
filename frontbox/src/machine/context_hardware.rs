//! The hardware namespaces on [`ServiceContext`] and [`SystemContext`]: `ctx.drivers`, `ctx.switches`, and `ctx.leds`.
//! Each reads the hardware lookup it derefs to, and sends commands to the hardware through [`MachineSystem`].
//!
//! Neither acts as the calling system, so they're the same on both contexts.

use std::ops::Deref;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::{GroupsExt, ROOT_GROUP};
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

/// `ctx.drivers`: look up drivers (through [`DriverLookup`]), and configure, activate, and deactivate them
#[derive(Clone, Copy)]
pub struct ContextDrivers<'a> {
  machine: MachineAccess<'a>,
}

impl<'a> ContextDrivers<'a> {
  pub(crate) fn new(
    base: &'a AppState,
    groups: &'a Groups,
    app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      machine: MachineAccess {
        base,
        groups,
        app_sender,
      },
    }
  }

  /// Configure a driver with a mode (DL). The driver has to be activated again to make it live. See
  /// [`MachineSystem::configure_driver`].
  pub fn configure(&self, driver: impl Into<&'static str>, mode: DriverMode) {
    self
      .machine
      .with(|machine, ctx| machine.configure_driver(driver.into(), mode, ctx));
  }

  /// Activate a driver (TL). See [`MachineSystem::activate_driver`].
  pub fn activate(&self, driver: impl Into<&'static str>, mode: ActivationMode) {
    self
      .machine
      .with(|machine, ctx| machine.activate_driver(driver.into(), mode, ctx));
  }

  /// Deactivate a driver (TL). See [`MachineSystem::deactivate_driver`].
  pub fn deactivate(&self, driver: impl Into<&'static str>, mode: DeactivationMode) {
    self
      .machine
      .with(|machine, ctx| machine.deactivate_driver(driver.into(), mode, ctx));
  }

  /// Whether the driver's current mode is triggered by a physical switch automatically. See
  /// [`MachineSystem::is_switch_triggered`].
  pub fn is_switch_triggered(&self, driver: &str) -> bool {
    self
      .machine
      .with(|machine, ctx| machine.is_switch_triggered(driver, ctx))
      .unwrap_or(false)
  }
}

impl Deref for ContextDrivers<'_> {
  type Target = DriverLookup;

  fn deref(&self) -> &Self::Target {
    &self.machine.base.drivers
  }
}

/// `ctx.switches`: look up switches and their state (through [`SwitchLookup`]), and configure how they report
#[derive(Clone, Copy)]
pub struct ContextSwitches<'a> {
  machine: MachineAccess<'a>,
}

impl<'a> ContextSwitches<'a> {
  pub(crate) fn new(
    base: &'a AppState,
    groups: &'a Groups,
    app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      machine: MachineAccess {
        base,
        groups,
        app_sender,
      },
    }
  }

  /// Configure how a switch reports (SL). See [`MachineSystem::configure_switch`].
  pub fn configure(
    &self,
    switch: &str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
  ) {
    self.machine.with(|machine, ctx| {
      machine.configure_switch(switch, inverted, debounce_close, debounce_open, ctx)
    });
  }
}

impl Deref for ContextSwitches<'_> {
  type Target = SwitchLookup;

  fn deref(&self) -> &Self::Target {
    &self.machine.base.switches
  }
}

/// `ctx.leds`: look up LEDs (through [`LedLookup`]), and set them directly
///
/// Setting an LED forces it to a color on the hardware right away. It isn't declared, so it isn't rendered, flattened,
/// or resolved against other declarations: if [`LedSystem`] is running and an LED is declared, the next render that
/// changes it writes over whatever was set here.
#[derive(Clone, Copy)]
pub struct ContextLeds<'a> {
  machine: MachineAccess<'a>,
}

impl<'a> ContextLeds<'a> {
  pub(crate) fn new(
    base: &'a AppState,
    groups: &'a Groups,
    app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      machine: MachineAccess {
        base,
        groups,
        app_sender,
      },
    }
  }

  /// Set the color of each LED, by index on the port/breakout, in a single command (RD)
  pub fn set(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u8, Rgba<u8>)>) {
    self
      .machine
      .with(|machine, _| machine.set_leds(expansion_id, breakout, led_states));
  }

  /// Set the LEDs at `indexes` on a port/breakout to the same color (RB)
  pub fn set_multiple(
    &self,
    expansion_id: u8,
    breakout: Option<u8>,
    rgba: Rgba<u8>,
    indexes: Vec<u8>,
  ) {
    self
      .machine
      .with(|machine, _| machine.set_multiple_leds(expansion_id, breakout, rgba, indexes));
  }

  /// Set every LED on a port/breakout to the same color (RA)
  pub fn set_all(&self, expansion_id: u8, breakout: Option<u8>, rgba: Rgba<u8>) {
    self
      .machine
      .with(|machine, _| machine.set_all_leds(expansion_id, breakout, rgba));
  }

  /// Set the white channel of each LED, by index on the port/breakout, in a single command (RZ)
  pub fn set_white(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u8, u8)>) {
    self
      .machine
      .with(|machine, _| machine.set_leds_white(expansion_id, breakout, led_states));
  }
}

impl Deref for ContextLeds<'_> {
  type Target = LedLookup;

  fn deref(&self) -> &Self::Target {
    &self.machine.base.leds
  }
}

/// What a hardware namespace holds to reach [`MachineSystem`]. It can't hold a [`ServiceContext`], which holds it, so
/// it builds one when it needs it.
#[derive(Clone, Copy)]
struct MachineAccess<'a> {
  base: &'a AppState,
  groups: &'a Groups,
  app_sender: &'a mpsc::UnboundedSender<AppMessage>,
}

impl MachineAccess<'_> {
  /// Run `f` on the machine, which the framework runs in the root group. Logs an error and returns `None` if it isn't
  /// running.
  fn with<R>(&self, f: impl FnOnce(&mut MachineSystem, &ServiceContext) -> R) -> Option<R> {
    let ctx = ServiceContext::new(self.base, self.groups, self.app_sender);
    let machine = self
      .groups
      .nearest(ROOT_GROUP, std::any::TypeId::of::<MachineSystem>())
      .and_then(|found| {
        self
          .groups
          .get(&found.group)?
          .get_by_type::<MachineSystem>()
      });
    match machine {
      Some(mut machine) => Some(f(&mut machine, &ctx)),
      None => {
        log::error!("Machine not running.");
        None
      }
    }
  }
}
