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
/// or resolved against other declarations: if `LedSystem` (in `frontbox-led`) is running and an LED is declared, the next render that
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

  /// Set the color of each LED, by index on the port/breakout, in a single command (RD). Each color is put into the
  /// channel order of its LED.
  pub fn set(&self, expansion_id: u8, breakout: Option<u8>, led_states: Vec<(u8, Rgb)>) {
    let leds = &self.machine.base.leds;
    let led_states = led_states
      .into_iter()
      .map(|(index, color)| {
        let channels = leds.channels_at(expansion_id, breakout, index);
        (index, channels.reorder(color))
      })
      .collect();
    self
      .machine
      .with(|machine, _| machine.set_leds(expansion_id, breakout, led_states));
  }

  /// Set the LEDs at `indexes` on a port/breakout to the same color (RB). LEDs that take their channels in different
  /// orders are sent in a command per order.
  pub fn set_multiple(&self, expansion_id: u8, breakout: Option<u8>, color: Rgb, indexes: Vec<u8>) {
    let leds = &self.machine.base.leds;
    let groups = group_by_channels(
      indexes
        .into_iter()
        .map(|index| (index, leds.channels_at(expansion_id, breakout, index))),
    );
    self.machine.with(|machine, _| {
      for (channels, indexes) in groups {
        machine.set_multiple_leds(expansion_id, breakout, channels.reorder(color), indexes);
      }
    });
  }

  /// Set every LED on a port/breakout to the same color (RA). The color is sent in the channel order most of its
  /// LEDs take, then LEDs that take another order are set again in theirs (RB).
  pub fn set_all(&self, expansion_id: u8, breakout: Option<u8>, color: Rgb) {
    let mut groups = group_by_channels(self.machine.base.leds.channels_on(expansion_id, breakout));
    groups.sort_by_key(|(_, indexes)| std::cmp::Reverse(indexes.len()));
    let base = groups
      .first()
      .map(|(channels, _)| *channels)
      .unwrap_or_default();

    self.machine.with(|machine, _| {
      machine.set_all_leds(expansion_id, breakout, base.reorder(color));
      for (channels, indexes) in groups.into_iter().skip(1) {
        machine.set_multiple_leds(expansion_id, breakout, channels.reorder(color), indexes);
      }
    });
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

/// Group LED indexes by the channel order their LEDs take, keeping the order each first appears in
fn group_by_channels(leds: impl Iterator<Item = (u8, LedChannels)>) -> Vec<(LedChannels, Vec<u8>)> {
  let mut groups: Vec<(LedChannels, Vec<u8>)> = Vec::new();
  for (index, channels) in leds {
    match groups.iter_mut().find(|(c, _)| *c == channels) {
      Some((_, indexes)) => indexes.push(index),
      None => groups.push((channels, vec![index])),
    }
  }
  groups
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_group_by_channels() {
    let leds = [
      (0, LedChannels::GRB),
      (1, LedChannels::RGB),
      (2, LedChannels::GRB),
      (3, LedChannels::BRG),
    ];
    assert_eq!(
      group_by_channels(leds.into_iter()),
      vec![
        (LedChannels::GRB, vec![0, 2]),
        (LedChannels::RGB, vec![1]),
        (LedChannels::BRG, vec![3]),
      ]
    );
  }
}
