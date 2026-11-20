//! The hardware namespaces on [`ServiceContext`] and [`SystemContext`]: `ctx.drivers`, `ctx.switches`, and `ctx.lights`.
//! Each reads the hardware lookup it derefs to, and sends commands to the hardware through [`MachineSystem`].
//!
//! Neither acts as the calling system, so they're the same on both contexts.

use std::ops::Deref;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::ROOT_GROUP;
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

/// `ctx.lights`: look up lights (through [`LightLookup`]), and set LEDs directly
///
/// Setting an LED forces it to a color on the hardware right away. It isn't declared, so it isn't rendered, flattened,
/// or resolved against other declarations: if `LightManager` (in `frontbox-lighting`) is running and an LED is declared, the next render that
/// changes it writes over whatever was set here.
#[derive(Clone, Copy)]
pub struct ContextLights<'a> {
  machine: MachineAccess<'a>,
}

impl<'a> ContextLights<'a> {
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
  pub fn set(&self, address: ExpAddress, led_states: Vec<(u8, Rgb)>) {
    let lights = &self.machine.base.lights;
    let led_states = led_states
      .into_iter()
      .map(|(index, color)| {
        let order = lights.order_at(address, index);
        (index, order.reorder(color))
      })
      .collect();
    self
      .machine
      .with(|machine, _| machine.set_leds(address, led_states));
  }

  /// Set the LEDs at `indexes` on a port/breakout to the same color (RB). LEDs that take their channels in different
  /// orders are sent in a command per order.
  pub fn set_multiple(&self, address: ExpAddress, color: Rgb, indexes: Vec<u8>) {
    let lights = &self.machine.base.lights;
    let groups = group_by_order(
      indexes
        .into_iter()
        .map(|index| (index, lights.order_at(address, index))),
    );
    self.machine.with(|machine, _| {
      for (order, indexes) in groups {
        machine.set_multiple_leds(address, order.reorder(color), indexes);
      }
    });
  }

  /// Set every LED on a port/breakout to the same color (RA). The color is sent in the channel order most of its
  /// LEDs take, then LEDs that take another order are set again in theirs (RB).
  pub fn set_all(&self, address: ExpAddress, color: Rgb) {
    let mut groups = group_by_order(self.machine.base.lights.orders_on(address));
    groups.sort_by_key(|(_, indexes)| std::cmp::Reverse(indexes.len()));
    let base = groups
      .first()
      .map(|(order, _)| *order)
      .unwrap_or_default();

    self.machine.with(|machine, _| {
      machine.set_all_leds(address, base.reorder(color));
      for (order, indexes) in groups.into_iter().skip(1) {
        machine.set_multiple_leds(address, order.reorder(color), indexes);
      }
    });
  }

  /// Set the white channel of each LED, by index on the port/breakout, in a single command (RZ)
  pub fn set_white(&self, address: ExpAddress, led_states: Vec<(u8, u8)>) {
    self
      .machine
      .with(|machine, _| machine.set_leds_white(address, led_states));
  }
}

impl Deref for ContextLights<'_> {
  type Target = LightLookup;

  fn deref(&self) -> &Self::Target {
    &self.machine.base.lights
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
  /// Run `f` on the machine, which the framework runs at the top level. Logs an error and returns `None` if it isn't
  /// running.
  fn with<R>(&self, f: impl FnOnce(&mut MachineSystem, &ServiceContext) -> R) -> Option<R> {
    let ctx = ServiceContext::new(self.base, self.groups, self.app_sender);
    let machine = self
      .groups
      .get(&ROOT_GROUP)
      .and_then(|root| root.get_by_type::<MachineSystem>());
    match machine {
      Some(mut machine) => Some(f(&mut machine, &ctx)),
      None => {
        log::error!(target: "frontbox::machine", "Machine not running.");
        None
      }
    }
  }
}

/// Group LED indexes by the channel order their LEDs take, keeping the order each first appears in
fn group_by_order(leds: impl Iterator<Item = (u8, ChannelOrder)>) -> Vec<(ChannelOrder, Vec<u8>)> {
  let mut groups: Vec<(ChannelOrder, Vec<u8>)> = Vec::new();
  for (index, order) in leds {
    match groups.iter_mut().find(|(o, _)| *o == order) {
      Some((_, indexes)) => indexes.push(index),
      None => groups.push((order, vec![index])),
    }
  }
  groups
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_group_by_order() {
    let leds = [
      (0, ChannelOrder::GRB),
      (1, ChannelOrder::RGB),
      (2, ChannelOrder::GRB),
      (3, ChannelOrder::BRG),
    ];
    assert_eq!(
      group_by_order(leds.into_iter()),
      vec![
        (ChannelOrder::GRB, vec![0, 2]),
        (ChannelOrder::RGB, vec![1]),
        (ChannelOrder::BRG, vec![3]),
      ]
    );
  }
}
