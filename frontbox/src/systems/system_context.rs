use std::ops::Deref;

use tokio::sync::mpsc;

use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;

/// # SystemContext
///
/// Each handler receives a reference to `SystemContext` which has both the `Context` and a reference to the `SystemHandle` of the current system. As this guide has shown, it's through Context that access several features is provided, including:
///
/// - Schedule cues
/// - Emit events
/// - Access hardware configuration and state
///
/// ```rust
/// # use frontbox::prelude::*;
/// # fn example(ctx: &SystemContext) {
/// let switches = &ctx.switches;
/// let drivers = &ctx.drivers;
/// let io_network = &ctx.io_network;
/// let exp_network = &ctx.exp_network;
/// # }
/// ```
///
/// The rest of what a system can do is grouped into namespaces, each acting as this system:
///
/// - `ctx.systems`: look up, spawn, and despawn systems
/// - `ctx.groups`: spawn and manage system groups
/// - `ctx.cues`: schedule signals back to this system
/// - `ctx.routines`: start and take part in routines
/// - `ctx.states`: read and set states
/// - `ctx.drivers`: look up, configure, activate, and deactivate drivers
/// - `ctx.switches`: look up switches and their state, and configure how they report
/// - `ctx.leds`: look up LEDs, and set them directly
///
/// **Important**: SystemContext is specific to the system it was given, and should not be shared to other systems.
pub struct SystemContext<'a> {
  handle: SystemHandle,
  ctx: ServiceContext<'a>,
  pub systems: ContextSystems<'a>,
  pub groups: ContextGroups<'a>,
  pub cues: ContextCues<'a>,
  pub routines: ContextRoutines<'a>,
  pub states: ContextStates<'a>,
  pub drivers: ContextDrivers<'a>,
  pub switches: ContextSwitches<'a>,
  pub leds: ContextLeds<'a>,
}

impl<'a> SystemContext<'a> {
  pub fn new(
    base: &'a AppState,
    handle: SystemHandle,
    groups: &'a Groups,
    app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    let ctx = ServiceContext::new(base, groups, app_sender);
    Self {
      handle,
      ctx,
      systems: ContextSystems::new(handle, ctx),
      groups: ContextGroups::new(handle, ctx),
      cues: ContextCues::new(handle, ctx),
      routines: ContextRoutines::new(handle, ctx),
      states: ContextStates::new(handle, ctx),
      drivers: ctx.drivers,
      switches: ctx.switches,
      leds: ctx.leds,
    }
  }

  /// Each system is automatically assigned an internal ID. This returns the ID for the current system. Pass this, not
  /// the handle or this context, to a public method of another system that needs to know who is calling it.
  pub fn id(&self) -> u64 {
    self.handle.id
  }

  /// The complete handle (group + ID) for the current system. A handle lets whoever holds it act as the system, so
  /// keep it to the system itself; see [`SystemHandle`].
  pub fn handle(&self) -> SystemHandle {
    self.handle
  }

  /// Dispatch an event to all other active systems
  pub fn emit<E: Event>(&self, event: E) {
    self.ctx.emit(self.id(), event);
  }

  /// Unregisters drivers, clears the EXP network, and halts the framework
  pub fn shutdown(&self, scope: ShutdownScope) {
    self.ctx.shutdown(scope);
  }
}

impl Deref for SystemContext<'_> {
  type Target = AppState;

  fn deref(&self) -> &Self::Target {
    self.ctx.base
  }
}

impl<'a> Into<ServiceContext<'a>> for SystemContext<'a> {
  fn into(self) -> ServiceContext<'a> {
    self.ctx
  }
}

impl<'a, 'b> Into<&'b ServiceContext<'a>> for &'b SystemContext<'a> {
  fn into(self) -> &'b ServiceContext<'a> {
    &self.ctx
  }
}
